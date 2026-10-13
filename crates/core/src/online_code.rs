//! Room codes for meeting online: made from a number the page supplies,
//! and tidied when a person types one. From Ribbit King's online.rs.

/// A room code from a number the page supplies (its clock): five letters and
/// digits with none that read alike.
pub fn room_code(seed: u64) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
    let mut r = crate::rng::Rng::new(seed);
    (0..5)
        .map(|_| ALPHABET[r.range(0, ALPHABET.len() as i32 - 1) as usize] as char)
        .collect()
}

/// A room code a person typed, tidied, or `None` if it cannot be one.
pub fn tidy_code(text: &str) -> Option<String> {
    let t: String = text
        .trim()
        .to_ascii_uppercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if (4..=12).contains(&t.len()) && t.chars().all(|c| c.is_ascii_alphanumeric()) {
        Some(t)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_codes_are_five_unambiguous_characters_and_typed_codes_are_tidied() {
        let a = room_code(1);
        assert_eq!(a.len(), 5);
        assert!(!a.contains('O') && !a.contains('0') && !a.contains('I') && !a.contains('1'));
        assert_ne!(room_code(1), room_code(2));
        assert_eq!(tidy_code(" ab3k9 "), Some("AB3K9".into()));
        assert_eq!(tidy_code("a-b"), None);
        assert_eq!(tidy_code("ab"), None);
    }
}
