pub const MIN_INTERVAL_MS: f64 = 1.0;
pub const MAX_INTERVAL_MS: f64 = 3_600_000.0;
pub const MAX_CPS: f64 = 1000.0 / MIN_INTERVAL_MS;
#[allow(dead_code)]
pub const SPEED_UNITS: [&str; 4] = ["ms", "sec", "min", "cps"];

pub fn allow_float(text: &str) -> bool {
    if text.len() > 12 {
        return false;
    }
    let mut dots = 0usize;
    let mut int_digits = 0usize;
    let mut frac_digits = 0usize;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            if dots == 0 {
                int_digits += 1;
                if int_digits > 8 {
                    return false;
                }
            } else {
                frac_digits += 1;
                if frac_digits > 3 {
                    return false;
                }
            }
        } else if ch == '.' && dots == 0 {
            dots = 1;
        } else {
            return false;
        }
    }
    true
}

pub fn allow_uint(text: &str) -> bool {
    text.len() <= 8 && text.chars().all(|c| c.is_ascii_digit())
}

pub fn allow_int(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return true;
    };
    let digit_count = if first == '-' {
        0
    } else if first.is_ascii_digit() {
        1
    } else {
        return false;
    };
    let rest_count = chars.clone().count();
    if digit_count + rest_count > 6 {
        return false;
    }
    chars.all(|c| c.is_ascii_digit())
}

pub fn parse_float(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let v = trimmed.parse::<f64>().ok()?;
    if v.is_finite() {
        Some(v)
    } else {
        None
    }
}

pub fn parse_int(text: &str) -> Option<i32> {
    parse_float(text).map(|v| v.round() as i32)
}

pub fn clamp_f64(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

pub fn to_interval_ms(value: f64, unit: &str) -> f64 {
    let ms = match unit {
        "cps" => {
            let safe = clamp_f64(value, 0.0001, MAX_CPS);
            1000.0 / safe
        }
        "sec" => value * 1000.0,
        "min" => value * 60_000.0,
        _ => value,
    };
    clamp_f64(ms, MIN_INTERVAL_MS, MAX_INTERVAL_MS)
}

pub fn from_interval_ms(ms: f64, unit: &str) -> f64 {
    let clamped = clamp_f64(ms, MIN_INTERVAL_MS, MAX_INTERVAL_MS);
    match unit {
        "cps" => 1000.0 / clamped,
        "sec" => clamped / 1000.0,
        "min" => clamped / 60_000.0,
        _ => clamped,
    }
}

pub fn sanitize_speed(text: &str, unit: &str) -> Option<f64> {
    let v = parse_float(text)?;
    if v <= 0.0 {
        return None;
    }
    let converted = from_interval_ms(to_interval_ms(v, unit), unit);
    let rounded = (converted * 1000.0).round() / 1000.0;
    if rounded > 0.0 {
        Some(rounded)
    } else {
        None
    }
}

pub fn fmt_number(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        return format!("{}", v.round() as i64);
    }
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn fmt_with_commas(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (idx, &b) in bytes.iter().enumerate() {
        if idx > 0 && (bytes.len() - idx).is_multiple_of(3) {
            out.push(',');
        }
        out.push(b as char);
    }
    out
}

pub fn effective_rate_hint(value: f64, unit: &str) -> String {
    let ms = to_interval_ms(value, unit);
    let cps = 1000.0 / ms;
    if unit == "cps" {
        if ms <= MIN_INTERVAL_MS + 1e-6 {
            "Base interval: 1 ms  (max 1,000 CPS)".to_string()
        } else {
            format!("Base interval: {} ms  (max 1,000 CPS)", fmt_number(ms))
        }
    } else if cps >= 1.0 {
        format!(
            "Effective rate: ~{} CPS  (min 1 ms)",
            fmt_with_commas(cps.round() as u64)
        )
    } else {
        format!("Effective rate: ~{:.2} CPS  (min 1 ms)", cps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_conversions_and_clamp() {
        assert!((to_interval_ms(10.0, "ms") - 10.0).abs() < 1e-6);
        assert!((to_interval_ms(0.5, "sec") - 500.0).abs() < 1e-6);
        assert!((to_interval_ms(1.0, "min") - 60_000.0).abs() < 1e-6);
        assert!((to_interval_ms(500.0, "cps") - 2.0).abs() < 1e-6);
        assert!((to_interval_ms(5000.0, "cps") - MIN_INTERVAL_MS).abs() < 1e-6);
        assert!((to_interval_ms(0.1, "ms") - MIN_INTERVAL_MS).abs() < 1e-6);
    }

    #[test]
    fn sanitize_and_format() {
        assert_eq!(sanitize_speed("10", "ms"), Some(10.0));
        assert_eq!(sanitize_speed("0", "ms"), None);
        assert_eq!(sanitize_speed("-5", "cps"), None);
        assert_eq!(sanitize_speed("abc", "ms"), None);
        assert_eq!(fmt_number(10.0), "10");
        assert_eq!(fmt_number(12.5), "12.5");
        assert_eq!(effective_rate_hint(10.0, "ms"), "Effective rate: ~100 CPS  (min 1 ms)");
    }

    #[test]
    fn input_filters() {
        assert!(allow_float("12.345"));
        assert!(!allow_float("12.3456"));
        assert!(!allow_float("-1"));
        assert!(allow_uint("123456"));
        assert!(!allow_uint("-5"));
        assert!(allow_int("-1920"));
        assert!(!allow_int("12-3"));
    }
}
