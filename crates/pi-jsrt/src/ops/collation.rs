//! Portable Unicode collation for the web Intl.Collator shim.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;

use icu_collator::options::{CaseLevel, CollatorOptions, Strength};
use icu_collator::preferences::{CollationCaseFirst, CollationNumericOrdering};
use icu_collator::{Collator, CollatorBorrowed, CollatorPreferences};
use icu_properties::props::{GeneralCategory, Script};
use icu_properties::CodePointMapData;

use crate::engine::{arg_string, OpFn};
use crate::js::{Context, JsArgs, JsResult, JsValue};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Sensitivity {
    Base,
    Accent,
    Case,
    Variant,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct CollatorKey {
    sensitivity: Sensitivity,
    case_first: CollationCaseFirst,
    numeric: bool,
}

thread_local! {
    static COLLATORS: RefCell<HashMap<CollatorKey, CollatorBorrowed<'static>>> = RefCell::new(HashMap::new());
    static DECIMAL_DIGIT_VALUES: RefCell<HashMap<(CollatorKey, char), Option<u8>>> = RefCell::new(HashMap::new());
}

fn collator(key: CollatorKey) -> CollatorBorrowed<'static> {
    let locale: icu_locale_core::Locale = "en-US"
        .parse()
        .expect("compiled ICU4X locale syntax");
    let mut prefs = CollatorPreferences::from(&locale);
    prefs.case_first = Some(key.case_first);
    prefs.numeric_ordering = Some(if key.numeric {
        CollationNumericOrdering::True
    } else {
        CollationNumericOrdering::False
    });
    let mut options = CollatorOptions::default();
    options.strength = Some(match key.sensitivity {
        Sensitivity::Base | Sensitivity::Case => Strength::Primary,
        Sensitivity::Accent => Strength::Secondary,
        Sensitivity::Variant => Strength::Tertiary,
    });
    if key.sensitivity == Sensitivity::Case {
        options.case_level = Some(CaseLevel::On);
    }
    Collator::try_new(prefs, options).expect("compiled ICU4X root collation data")
}

fn is_decimal_digit(ch: char) -> bool {
    CodePointMapData::<GeneralCategory>::new().get(ch) == GeneralCategory::DecimalNumber
}

fn is_astral_common_decimal(ch: char) -> bool {
    ch as u32 > 0xffff
        && is_decimal_digit(ch)
        && CodePointMapData::<Script>::new().get(ch) == Script::Common
}

fn is_common_decimal(ch: char) -> bool {
    is_decimal_digit(ch) && CodePointMapData::<Script>::new().get(ch) == Script::Common
}

fn is_greek_uppercase(ch: char) -> bool {
    CodePointMapData::<Script>::new().get(ch) == Script::Greek
        && CodePointMapData::<GeneralCategory>::new().get(ch) == GeneralCategory::UppercaseLetter
}

fn scalar_at(units: &[u16], index: usize) -> Option<char> {
    char::decode_utf16(units.get(index..)?.iter().copied())
        .next()?
        .ok()
}

fn previous_scalar(units: &[u16], index: usize) -> Option<char> {
    char::decode_utf16(units.get(..index)?.iter().copied())
        .last()?
        .ok()
}

fn decimal_digit_value(ch: char, key: CollatorKey) -> Option<u8> {
    if !key.numeric || !is_decimal_digit(ch) {
        return None;
    }
    DECIMAL_DIGIT_VALUES.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(value) = cache.get(&(key, ch)) {
            return *value;
        }
        let mut encoded = [0u16; 2];
        let encoded = ch.encode_utf16(&mut encoded);
        let value = (0..=9).find(|candidate| {
            let ascii = [b'0' as u16 + *candidate as u16];
            collate(key, encoded, &ascii) == Ordering::Equal
        });
        cache.insert((key, ch), value);
        value
    })
}

fn decimal_digit_run(units: &[u16], index: usize, key: CollatorKey) -> Option<(usize, Vec<u8>)> {
    let mut end = index;
    let mut digits = Vec::new();
    while let Some(ch) = scalar_at(units, end) {
        let Some(value) = decimal_digit_value(ch, key) else {
            break;
        };
        digits.push(value);
        end += ch.len_utf16();
    }
    (!digits.is_empty()).then_some((end, digits))
}

fn ascii_digit_run(units: &[u16], index: usize) -> usize {
    units[index..]
        .iter()
        .position(|unit| !(b'0' as u16..=b'9' as u16).contains(unit))
        .map_or(units.len(), |offset| index + offset)
}

fn significant_ascii_digits(digits: &[u16]) -> &[u16] {
    let first_nonzero = digits
        .iter()
        .position(|digit| *digit != b'0' as u16)
        .unwrap_or(digits.len().saturating_sub(1));
    &digits[first_nonzero..]
}

fn compare_ascii_digit_values(left: &[u16], right: &[u16]) -> Ordering {
    let left = significant_ascii_digits(left);
    let right = significant_ascii_digits(right);
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn significant_digits(digits: &[u8]) -> &[u8] {
    let first_nonzero = digits
        .iter()
        .position(|digit| *digit != 0)
        .unwrap_or(digits.len().saturating_sub(1));
    &digits[first_nonzero..]
}

fn compare_digit_values(left: &[u8], right: &[u8]) -> Ordering {
    let left = significant_digits(left);
    let right = significant_digits(right);
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn collate(key: CollatorKey, left: &[u16], right: &[u16]) -> Ordering {
    COLLATORS.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache
            .entry(key)
            .or_insert_with(|| collator(key))
            .compare_utf16(left, right)
    })
}

fn compare(a: &str, b: &str, key: CollatorKey) -> Ordering {
    let left: Vec<u16> = a.encode_utf16().collect();
    let right: Vec<u16> = b.encode_utf16().collect();
    let (mut i, mut j) = (0, 0);
    let (mut localized_i, mut localized_j) = (0, 0);
    let mut numeric_tie = Ordering::Equal;

    while i < left.len() && j < right.len() {
        let left_ascii_digit = (b'0' as u16..=b'9' as u16).contains(&left[i]);
        let right_ascii_digit = (b'0' as u16..=b'9' as u16).contains(&right[j]);
        if left_ascii_digit && right_ascii_digit {
            let (left_start, right_start) = (i, j);
            i = ascii_digit_run(&left, i);
            j = ascii_digit_run(&right, j);
            let value_order = compare_ascii_digit_values(
                &left[left_start..i],
                &right[right_start..j],
            );
            if value_order != Ordering::Equal {
                return value_order;
            }
            if numeric_tie == Ordering::Equal {
                numeric_tie = (i - left_start).cmp(&(j - right_start));
            }
            continue;
        }
        let left_scalar = scalar_at(&left, i);
        let right_scalar = scalar_at(&right, j);
        if let (Some(left_char), Some(right_char)) = (left_scalar, right_scalar) {
            if left_char == right_char {
                let left_width = left_char.len_utf16();
                let right_width = right_char.len_utf16();
                i += left_width;
                j += right_width;
                if left_width > 1 {
                    localized_i = i;
                    localized_j = j;
                }
                continue;
            }
            if !left_char.is_ascii()
                && !right_char.is_ascii()
                && is_common_decimal(left_char)
                && is_common_decimal(right_char)
            {
                let (_, left_digits) = decimal_digit_run(&left, i, key).expect("left common Nd");
                let (_, right_digits) = decimal_digit_run(&right, j, key).expect("right common Nd");
                let value_order = compare_digit_values(&left_digits, &right_digits);
                if value_order != Ordering::Equal {
                    return value_order;
                }
            }
            let previous_left_is_decimal = previous_scalar(&left, i).is_some_and(is_decimal_digit);
            let previous_right_is_decimal =
                previous_scalar(&right, j).is_some_and(is_decimal_digit);
            let left_is_astral_decimal = left_scalar.is_some_and(is_astral_common_decimal);
            let right_is_astral_decimal = right_scalar.is_some_and(is_astral_common_decimal);
            let left_is_decimal = left_scalar.is_some_and(is_decimal_digit);
            let right_is_decimal = right_scalar.is_some_and(is_decimal_digit);
            let left_greek_uppercase_before_decimal = left_scalar.is_some_and(|ch| {
                is_greek_uppercase(ch)
                    && scalar_at(&left, i + ch.len_utf16()).is_some_and(is_decimal_digit)
            });
            let right_greek_uppercase_before_decimal = right_scalar.is_some_and(|ch| {
                is_greek_uppercase(ch)
                    && scalar_at(&right, j + ch.len_utf16()).is_some_and(is_decimal_digit)
            });
            let split_at_digit_boundary = previous_left_is_decimal
                && previous_right_is_decimal
                && ((left_is_astral_decimal && !right_is_decimal)
                    || (right_is_astral_decimal && !left_is_decimal))
                && !left_greek_uppercase_before_decimal
                && !right_greek_uppercase_before_decimal;
            let left_start = if split_at_digit_boundary { i } else { localized_i };
            let right_start = if split_at_digit_boundary { j } else { localized_j };
            let order = collate(key, &left[left_start..], &right[right_start..]);
            if order != Ordering::Equal {
                return order;
            }
            break;
        }
    }

    if i == left.len() && j < right.len() {
        return Ordering::Less;
    }
    if j == right.len() && i < left.len() {
        return Ordering::Greater;
    }
    if numeric_tie != Ordering::Equal {
        return numeric_tie;
    }
    left.cmp(&right)
}

fn op_intl_collation_compare(
    _: &JsValue,
    args: &[JsValue],
    ctx: &mut Context,
) -> JsResult<JsValue> {
    let a = arg_string(args, 0, ctx)?;
    let b = arg_string(args, 1, ctx)?;
    let sensitivity = match args
        .get_or_undefined(2)
        .to_string(ctx)?
        .to_std_string_lossy()
        .as_str()
    {
        "base" => Sensitivity::Base,
        "accent" => Sensitivity::Accent,
        "case" => Sensitivity::Case,
        "variant" => Sensitivity::Variant,
        _ => return Err(crate::engine::type_error("Invalid collation sensitivity")),
    };
    let numeric = args.get_or_undefined(3).to_boolean();
    let case_first = match args
        .get_or_undefined(4)
        .to_string(ctx)?
        .to_std_string_lossy()
        .as_str()
    {
        "false" => CollationCaseFirst::False,
        "lower" => CollationCaseFirst::Lower,
        "upper" => CollationCaseFirst::Upper,
        _ => return Err(crate::engine::type_error("Invalid collation caseFirst")),
    };
    let order = compare(
        &a,
        &b,
        CollatorKey {
            sensitivity,
            case_first,
            numeric,
        },
    );
    Ok(JsValue::from(match order {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }))
}

pub(crate) const OPS: &[(&str, usize, OpFn)] =
    &[("intlCollationCompare", 5, op_intl_collation_compare)];
