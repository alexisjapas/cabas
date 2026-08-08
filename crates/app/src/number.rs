//! Text in, exact rational out — and back again for display.
//!
//! This is the border Rule 4 draws. Everything inward of it is a
//! [`Rational`] and stays exact; everything outward is a string a person
//! reads. **No `f64` on either path**: parsing through `f64` would put a
//! rounding step between what the cook typed and what the cart adds up, which
//! is the whole failure the rule exists to prevent.
//!
//! Rendering, unlike parsing, cannot always be exact — a quantity that came
//! out of an imperial conversion is `28349523125/1000000000` grams and no
//! human wants to read it. So a rendered amount says whether it is the exact
//! value or a rounded one, and the UI can mark the difference. Two honest
//! lines beat one invented number (Rule 5), and the same honesty applies to
//! one line.

use cabas_domain::Rational;

use crate::error::{AppError, Result};

/// The denominators a recipe is actually written in: halves, thirds,
/// quarters, and their subdivisions. These render as fractions — "1 1/2" is
/// how a cook writes it and "1.5" is how a spreadsheet does.
///
/// Tenths and fifths are deliberately absent even though they are small:
/// 1300 g of flour is "1.3 kg" to everyone alive, and "1 3/10 kg" to nobody.
const FRACTIONS: [i128; 7] = [2, 3, 4, 6, 8, 12, 16];

/// Decimals kept when a value cannot be a tidy fraction. Two is where a
/// kitchen scale stops caring.
const DECIMALS: i128 = 100;

/// A rendered amount, and whether anything was lost rendering it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Rendered {
    pub text: String,
    pub approximate: bool,
}

/// Reads an amount a person typed: `200`, `1.5`, `1,5`, `1/2`, `1 1/2`.
///
/// The comma is accepted because the app is used in French and that is what
/// the keyboard offers; accepting it here rather than normalising it in the
/// UI keeps Rule 9 intact — the frontend passes the field through untouched.
pub(crate) fn parse_amount(field: &'static str, raw: &str) -> Result<Rational> {
    let text = raw.trim().replace(',', ".");
    let mut parts = text.split_whitespace();
    let value = match (parts.next(), parts.next(), parts.next()) {
        // "1 1/2" — a whole part and a fraction, the way a recipe writes it.
        (Some(whole), Some(fraction), None) => match (integer(whole), fraction_of(fraction)) {
            (Some(w), Some(f)) => Some(w + f),
            _ => None,
        },
        (Some(single), None, None) => term(single),
        _ => None,
    };

    let value = value.ok_or_else(|| {
        AppError::invalid(
            field,
            format!("{raw:?} is not an amount — try 200, 1.5 or 1/2"),
        )
    })?;
    if *value.numer() <= 0 {
        return Err(AppError::invalid(
            field,
            "an amount must be greater than zero",
        ));
    }
    Ok(value)
}

fn term(text: &str) -> Option<Rational> {
    match text.split_once('/') {
        Some(_) => fraction_of(text),
        None => decimal(text),
    }
}

fn fraction_of(text: &str) -> Option<Rational> {
    let (numer, denom) = text.split_once('/')?;
    let numer = integer(numer)?;
    let denom = integer(denom)?;
    if *denom.numer() == 0 {
        return None;
    }
    Some(numer / denom)
}

fn integer(text: &str) -> Option<Rational> {
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse::<i128>().ok())
        .flatten()
        .map(Rational::from_integer)
}

/// `"1.5"` → 3/2, exactly. The denominator is a power of ten, so nothing is
/// approximated on the way in.
fn decimal(text: &str) -> Option<Rational> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    if !whole
        .bytes()
        .chain(fraction.bytes())
        .all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let scale = 10i128.checked_pow(u32::try_from(fraction.len()).ok()?)?;
    let whole: i128 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let fraction: i128 = if fraction.is_empty() {
        0
    } else {
        fraction.parse().ok()?
    };
    Some(Rational::new(
        whole.checked_mul(scale)?.checked_add(fraction)?,
        scale,
    ))
}

/// The way round a person reads it.
///
/// Whole numbers stay whole, small denominators stay fractions, and anything
/// else becomes a two-decimal approximation that says so.
pub(crate) fn render(value: Rational) -> Rendered {
    let (numer, denom) = (*value.numer(), *value.denom());
    if denom == 1 {
        return exact(numer.to_string());
    }
    if FRACTIONS.contains(&denom) {
        let sign = if numer < 0 { "-" } else { "" };
        let (whole, rest) = (numer.abs() / denom, numer.abs() % denom);
        return exact(if whole == 0 {
            format!("{sign}{rest}/{denom}")
        } else {
            format!("{sign}{whole} {rest}/{denom}")
        });
    }

    // Exact until proven otherwise: 1/20 is 0.05 and nothing is lost, while
    // 1/3 is not and the caller deserves to know. The multiplication reduces
    // before it widens, so an overflow here would need a quantity no kitchen
    // has ever produced.
    let scaled = value * Rational::from_integer(DECIMALS);
    let rounded = scaled.round();
    let hundredths = rounded.to_integer();
    let sign = if hundredths < 0 { "-" } else { "" };
    let (whole, cents) = (hundredths.abs() / DECIMALS, hundredths.abs() % DECIMALS);
    let text = match cents {
        0 => format!("{sign}{whole}"),
        c if c % 10 == 0 => format!("{sign}{whole}.{}", c / 10),
        c => format!("{sign}{whole}.{c:02}"),
    };
    Rendered {
        text,
        approximate: rounded != scaled,
    }
}

/// The same value, rendered so that parsing it back cannot lose anything.
///
/// For an edit form, not for a screen. [`render`] rounds when a value has no
/// tidy form, and an editor that showed the rounded text would write it back
/// on the next save: open a recipe, change its name, and a quantity silently
/// becomes 28.35 g. So when the pretty rendering is approximate this falls
/// back to the raw fraction, which is ugly and exact — and only ever appears
/// on values that came out of an imperial conversion.
pub(crate) fn render_lossless(value: Rational) -> String {
    let rendered = render(value);
    if rendered.approximate {
        format!("{}/{}", value.numer(), value.denom())
    } else {
        rendered.text
    }
}

fn exact(text: String) -> Rendered {
    Rendered {
        text,
        approximate: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(raw: &str) -> Rational {
        parse_amount("amount", raw).expect("valid amount")
    }

    #[test]
    fn every_form_a_cook_types_parses_exactly() {
        assert_eq!(parsed("200"), Rational::from_integer(200));
        assert_eq!(parsed("1.5"), Rational::new(3, 2));
        // The French keyboard's comma.
        assert_eq!(parsed("1,5"), Rational::new(3, 2));
        assert_eq!(parsed("1/2"), Rational::new(1, 2));
        assert_eq!(parsed("1 1/2"), Rational::new(3, 2));
        assert_eq!(parsed("  0.25 "), Rational::new(1, 4));
        assert_eq!(parsed(".5"), Rational::new(1, 2));
    }

    #[test]
    fn a_third_typed_as_a_fraction_stays_a_third() {
        // The property Rule 4 exists for: parsing through f64 would give
        // 0.3333333333333333 and three of them would not be one.
        let third = parsed("1/3");
        assert_eq!(third * Rational::from_integer(3), Rational::from_integer(1));
    }

    #[test]
    fn nonsense_and_non_positive_amounts_are_refused() {
        for bad in [
            "", "  ", "one", "1.5.2", "1/0", "-3", "0", "0.0", "1 2 3", "1/2/3", "1e3",
        ] {
            assert!(
                parse_amount("amount", bad).is_err(),
                "{bad:?} should not parse"
            );
        }
    }

    #[test]
    fn whole_numbers_and_kitchen_fractions_render_exactly() {
        assert_eq!(render(Rational::from_integer(200)).text, "200");
        assert_eq!(render(Rational::new(3, 2)).text, "1 1/2");
        assert_eq!(render(Rational::new(1, 3)).text, "1/3");
        assert_eq!(render(Rational::new(2, 3)).text, "2/3");
        assert!(!render(Rational::new(1, 3)).approximate);
    }

    #[test]
    fn tenths_are_decimals_and_not_fractions() {
        // 1300 g of flour, humanised into kilograms. "1 3/10 kg" is how
        // nobody writes it.
        assert_eq!(render(Rational::new(13, 10)).text, "1.3");
        assert_eq!(render(Rational::new(1, 5)).text, "0.2");
        assert!(!render(Rational::new(13, 10)).approximate);
    }

    #[test]
    fn awkward_denominators_become_decimals_that_admit_it() {
        // What an imperial conversion leaves behind: 1 oz in grams.
        let ounce = Rational::new(28_349_523_125, 1_000_000_000);
        let rendered = render(ounce);
        assert_eq!(rendered.text, "28.35");
        assert!(rendered.approximate, "a rounded amount must say so");

        // A hundredth is not a nice fraction but is an exact decimal.
        let exact = render(Rational::new(1, 20));
        assert_eq!(exact.text, "0.05");
        assert!(!exact.approximate);
    }

    #[test]
    fn rendering_round_trips_through_parsing_when_it_is_exact() {
        for value in ["200", "1.5", "1/3", "1 1/2", "0.05"] {
            let parsed = parsed(value);
            let rendered = render(parsed);
            assert!(!rendered.approximate);
            assert_eq!(parsed_or_panic(&rendered.text), parsed, "{value}");
        }
    }

    fn parsed_or_panic(raw: &str) -> Rational {
        parse_amount("amount", raw).expect("a rendered exact amount must re-parse")
    }

    #[test]
    fn an_edit_form_never_loses_precision_to_its_own_rendering() {
        // The bug this exists for: open a recipe to fix a typo, save it, and
        // an ounce-derived quantity has quietly become 28.35 g.
        let ounce = Rational::new(28_349_523_125, 1_000_000_000);
        let text = render_lossless(ounce);
        assert_eq!(parsed_or_panic(&text), ounce);
        // The pretty form stays pretty when it can.
        assert_eq!(render_lossless(Rational::new(3, 2)), "1 1/2");
        assert_eq!(render_lossless(Rational::from_integer(200)), "200");
    }
}
