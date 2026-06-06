//! Custom single-byte character encoding for the trie model.
//!
//! Slot layout:
//!   0        = sentinel (whitespace / dash / empty / no-char)
//!   1–26     = a–z
//!   27       = _
//!   28–37    = 0–9
//!   38–106   = Latin extended (accented: French, Spanish, German, Polish, Czech…)
//!   107–131  = Greek lowercase (α–ω)
//!   132–163  = Cyrillic lowercase (а–я)
//!   164–169  = Extra Cyrillic (ё ї і є ў ґ)
//!   170–205  = Arabic base letters
//!   206–248  = Georgian Mkhedruli
//!   249–255  = reserved

#![no_std]

/// Normalise a `char` to its canonical lowercase form before encoding.
/// Handles ASCII, Latin extended, Greek, Cyrillic, Arabic (no case), Georgian (no case).
#[inline]
#[rustfmt::skip]
pub fn normalise(ch: char) -> char {
    // Fast path for ASCII
    if ch.is_ascii() {
        return ch.to_ascii_lowercase();
    }
    // Latin extended uppercase: U+00C0–U+00D6, U+00D8–U+00DE -> +0x20
    if ('\u{00C0}'..='\u{00D6}').contains(&ch) || ('\u{00D8}'..='\u{00DE}').contains(&ch) {
        return char::from_u32(ch as u32 + 0x20).unwrap_or(ch);
    }
    // Latin extended block 2 (uppercase even codepoints -> lowercase odd, roughly)
    // Handle common ones explicitly via a small table
    match ch {
        'Ā' => 'ā', 'Ă' => 'ă', 'Ą' => 'ą',
        'Ć' => 'ć', 'Č' => 'č', 'Ď' => 'ď', 'Đ' => 'đ',
        'Ē' => 'ē', 'Ě' => 'ě', 'Ę' => 'ę',
        'Ğ' => 'ğ', 'Ģ' => 'ģ', 'Ħ' => 'ħ',
        'Ī' => 'ī', 'Į' => 'į',
        'Ķ' => 'ķ', 'Ĺ' => 'ĺ', 'Ļ' => 'ļ', 'Ľ' => 'ľ', 'Ł' => 'ł',
        'Ń' => 'ń', 'Ņ' => 'ņ', 'Ň' => 'ň',
        'Œ' => 'œ', 'Ŕ' => 'ŕ', 'Ř' => 'ř',
        'Ś' => 'ś', 'Ş' => 'ş', 'Š' => 'š',
        'Ť' => 'ť', 'Ţ' => 'ţ',
        'Ū' => 'ū', 'Ů' => 'ů', 'Ű' => 'ű', 'Ų' => 'ų',
        'Ź' => 'ź', 'Ż' => 'ż', 'Ž' => 'ž',
        // Greek uppercase Α–Ω (U+0391–U+03A9) -> +0x20
        '\u{0391}'..='\u{03A9}' => char::from_u32(ch as u32 + 0x20).unwrap_or(ch),
        // Cyrillic uppercase А–Я (U+0410–U+042F) -> +0x20
        '\u{0410}'..='\u{042F}' => char::from_u32(ch as u32 + 0x20).unwrap_or(ch),
        'Ё' => 'ё', 'Ї' => 'ї', 'І' => 'і', 'Є' => 'є', 'Ў' => 'ў', 'Ґ' => 'ґ',
        // Arabic and Georgian have no case distinction
        _ => ch,
    }
}

/// Encode a `char` to its slot byte.
/// Returns `None` for unsupported characters.
/// Call `normalise` first if input may be uppercase.
#[inline]
#[rustfmt::skip]
pub fn encode(ch: char) -> Option<u8> {
    match ch {
        // a–z
        'a' => Some(1),  'b' => Some(2),  'c' => Some(3),  'd' => Some(4),
        'e' => Some(5),  'f' => Some(6),  'g' => Some(7),  'h' => Some(8),
        'i' => Some(9),  'j' => Some(10), 'k' => Some(11), 'l' => Some(12),
        'm' => Some(13), 'n' => Some(14), 'o' => Some(15), 'p' => Some(16),
        'q' => Some(17), 'r' => Some(18), 's' => Some(19), 't' => Some(20),
        'u' => Some(21), 'v' => Some(22), 'w' => Some(23), 'x' => Some(24),
        'y' => Some(25), 'z' => Some(26),
        // special
        '_' => Some(27),
        // 0–9
        '0' => Some(28), '1' => Some(29), '2' => Some(30), '3' => Some(31),
        '4' => Some(32), '5' => Some(33), '6' => Some(34), '7' => Some(35),
        '8' => Some(36), '9' => Some(37),
        // Latin extended
        'à' => Some(38),  'á' => Some(39),  'â' => Some(40),  'ã' => Some(41),
        'ä' => Some(42),  'å' => Some(43),  'æ' => Some(44),  'ç' => Some(45),
        'è' => Some(46),  'é' => Some(47),  'ê' => Some(48),  'ë' => Some(49),
        'ì' => Some(50),  'í' => Some(51),  'î' => Some(52),  'ï' => Some(53),
        'ð' => Some(54),  'ñ' => Some(55),  'ò' => Some(56),  'ó' => Some(57),
        'ô' => Some(58),  'õ' => Some(59),  'ö' => Some(60),  'ø' => Some(61),
        'ù' => Some(62),  'ú' => Some(63),  'û' => Some(64),  'ü' => Some(65),
        'ý' => Some(66),  'þ' => Some(67),  'ÿ' => Some(68),
        'ā' => Some(69),  'ă' => Some(70),  'ą' => Some(71),
        'ć' => Some(72),  'č' => Some(73),  'ď' => Some(74),  'đ' => Some(75),
        'ē' => Some(76),  'ě' => Some(77),  'ę' => Some(78),
        'ğ' => Some(79),  'ģ' => Some(80),  'ħ' => Some(81),
        'ī' => Some(82),  'į' => Some(83),  'ķ' => Some(84),
        'ĺ' => Some(85),  'ļ' => Some(86),  'ľ' => Some(87),  'ł' => Some(88),
        'ń' => Some(89),  'ņ' => Some(90),  'ň' => Some(91),  'œ' => Some(92),
        'ŕ' => Some(93),  'ř' => Some(94),  'ś' => Some(95),  'ş' => Some(96),
        'š' => Some(97),  'ť' => Some(98),  'ţ' => Some(99),
        'ū' => Some(100), 'ů' => Some(101), 'ű' => Some(102), 'ų' => Some(103),
        'ź' => Some(104), 'ż' => Some(105), 'ž' => Some(106),
        // Greek
        'α' => Some(107), 'β' => Some(108), 'γ' => Some(109), 'δ' => Some(110),
        'ε' => Some(111), 'ζ' => Some(112), 'η' => Some(113), 'θ' => Some(114),
        'ι' => Some(115), 'κ' => Some(116), 'λ' => Some(117), 'μ' => Some(118),
        'ν' => Some(119), 'ξ' => Some(120), 'ο' => Some(121), 'π' => Some(122),
        'ρ' => Some(123), 'ς' => Some(124), 'σ' => Some(125), 'τ' => Some(126),
        'υ' => Some(127), 'φ' => Some(128), 'χ' => Some(129), 'ψ' => Some(130),
        'ω' => Some(131),
        // Cyrillic
        'а' => Some(132), 'б' => Some(133), 'в' => Some(134), 'г' => Some(135),
        'д' => Some(136), 'е' => Some(137), 'ж' => Some(138), 'з' => Some(139),
        'и' => Some(140), 'й' => Some(141), 'к' => Some(142), 'л' => Some(143),
        'м' => Some(144), 'н' => Some(145), 'о' => Some(146), 'п' => Some(147),
        'р' => Some(148), 'с' => Some(149), 'т' => Some(150), 'у' => Some(151),
        'ф' => Some(152), 'х' => Some(153), 'ц' => Some(154), 'ч' => Some(155),
        'ш' => Some(156), 'щ' => Some(157), 'ъ' => Some(158), 'ы' => Some(159),
        'ь' => Some(160), 'э' => Some(161), 'ю' => Some(162), 'я' => Some(163),
        // Extra Cyrillic
        'ё' => Some(164), 'ї' => Some(165), 'і' => Some(166),
        'є' => Some(167), 'ў' => Some(168), 'ґ' => Some(169),
        // Arabic
        'ء' => Some(170), 'آ' => Some(171), 'أ' => Some(172), 'ؤ' => Some(173),
        'إ' => Some(174), 'ئ' => Some(175), 'ا' => Some(176), 'ب' => Some(177),
        'ة' => Some(178), 'ت' => Some(179), 'ث' => Some(180), 'ج' => Some(181),
        'ح' => Some(182), 'خ' => Some(183), 'د' => Some(184), 'ذ' => Some(185),
        'ر' => Some(186), 'ز' => Some(187), 'س' => Some(188), 'ش' => Some(189),
        'ص' => Some(190), 'ض' => Some(191), 'ط' => Some(192), 'ظ' => Some(193),
        'ع' => Some(194), 'غ' => Some(195), 'ف' => Some(196), 'ق' => Some(197),
        'ك' => Some(198), 'ل' => Some(199), 'م' => Some(200), 'ن' => Some(201),
        'ه' => Some(202), 'و' => Some(203), 'ى' => Some(204), 'ي' => Some(205),
        // Georgian
        'ა' => Some(206), 'ბ' => Some(207), 'გ' => Some(208), 'დ' => Some(209),
        'ე' => Some(210), 'ვ' => Some(211), 'ზ' => Some(212), 'თ' => Some(213),
        'ი' => Some(214), 'კ' => Some(215), 'ლ' => Some(216), 'მ' => Some(217),
        'ნ' => Some(218), 'ო' => Some(219), 'პ' => Some(220), 'ჟ' => Some(221),
        'რ' => Some(222), 'ს' => Some(223), 'ტ' => Some(224), 'უ' => Some(225),
        'ფ' => Some(226), 'ქ' => Some(227), 'ღ' => Some(228), 'ყ' => Some(229),
        'შ' => Some(230), 'ჩ' => Some(231), 'ც' => Some(232), 'ძ' => Some(233),
        'წ' => Some(234), 'ჭ' => Some(235), 'ხ' => Some(236), 'ჯ' => Some(237),
        'ჰ' => Some(238), 'ჱ' => Some(239), 'ჲ' => Some(240), 'ჳ' => Some(241),
        'ჴ' => Some(242), 'ჵ' => Some(243), 'ჶ' => Some(244), 'ჷ' => Some(245),
        'ჸ' => Some(246), 'ჹ' => Some(247), 'ჺ' => Some(248),
        _ => None,
    }
}

/// Encode a `char`, normalising case first.
#[inline]
pub fn encode_normalised(ch: char) -> Option<u8> {
    encode(normalise(ch))
}

/// Decode a slot byte back to its canonical `char`.
/// Slot 0 (sentinel) returns `None`.
#[inline]
pub fn decode(slot: u8) -> Option<char> {
    #[rustfmt::skip]
    const T: [Option<char>; 256] = [
        None,         Some('a'),  Some('b'),  Some('c'),  Some('d'),  Some('e'),  // 0-5
        Some('f'),    Some('g'),  Some('h'),  Some('i'),  Some('j'),  Some('k'),  // 6-11
        Some('l'),    Some('m'),  Some('n'),  Some('o'),  Some('p'),  Some('q'),  // 12-17
        Some('r'),    Some('s'),  Some('t'),  Some('u'),  Some('v'),  Some('w'),  // 18-23
        Some('x'),    Some('y'),  Some('z'),  Some('_'),                          // 24-27
        Some('0'),    Some('1'),  Some('2'),  Some('3'),  Some('4'),  Some('5'),  // 28-33
        Some('6'),    Some('7'),  Some('8'),  Some('9'),                          // 34-37
        Some('à'),    Some('á'),  Some('â'),  Some('ã'),  Some('ä'),  Some('å'),  // 38-43
        Some('æ'),    Some('ç'),  Some('è'),  Some('é'),  Some('ê'),  Some('ë'),  // 44-49
        Some('ì'),    Some('í'),  Some('î'),  Some('ï'),  Some('ð'),  Some('ñ'),  // 50-55
        Some('ò'),    Some('ó'),  Some('ô'),  Some('õ'),  Some('ö'),  Some('ø'),  // 56-61
        Some('ù'),    Some('ú'),  Some('û'),  Some('ü'),  Some('ý'),  Some('þ'),  // 62-67
        Some('ÿ'),    Some('ā'),  Some('ă'),  Some('ą'),  Some('ć'),  Some('č'),  // 68-73
        Some('ď'),    Some('đ'),  Some('ē'),  Some('ě'),  Some('ę'),  Some('ğ'),  // 74-79
        Some('ģ'),    Some('ħ'),  Some('ī'),  Some('į'),  Some('ķ'),  Some('ĺ'),  // 80-85
        Some('ļ'),    Some('ľ'),  Some('ł'),  Some('ń'),  Some('ņ'),  Some('ň'),  // 86-91
        Some('œ'),    Some('ŕ'),  Some('ř'),  Some('ś'),  Some('ş'),  Some('š'),  // 92-97
        Some('ť'),    Some('ţ'),  Some('ū'),  Some('ů'),  Some('ű'),  Some('ų'),  // 98-103
        Some('ź'),    Some('ż'),  Some('ž'),                                      // 104-106
        Some('α'),    Some('β'),  Some('γ'),  Some('δ'),  Some('ε'),  Some('ζ'),  // 107-112
        Some('η'),    Some('θ'),  Some('ι'),  Some('κ'),  Some('λ'),  Some('μ'),  // 113-118
        Some('ν'),    Some('ξ'),  Some('ο'),  Some('π'),  Some('ρ'),  Some('ς'),  // 119-124
        Some('σ'),    Some('τ'),  Some('υ'),  Some('φ'),  Some('χ'),  Some('ψ'),  // 125-130
        Some('ω'),                                                                 // 131
        Some('а'),    Some('б'),  Some('в'),  Some('г'),  Some('д'),  Some('е'),  // 132-137
        Some('ж'),    Some('з'),  Some('и'),  Some('й'),  Some('к'),  Some('л'),  // 138-143
        Some('м'),    Some('н'),  Some('о'),  Some('п'),  Some('р'),  Some('с'),  // 144-149
        Some('т'),    Some('у'),  Some('ф'),  Some('х'),  Some('ц'),  Some('ч'),  // 150-155
        Some('ш'),    Some('щ'),  Some('ъ'),  Some('ы'),  Some('ь'),  Some('э'),  // 156-161
        Some('ю'),    Some('я'),                                                   // 162-163
        Some('ё'),    Some('ї'),  Some('і'),  Some('є'),  Some('ў'),  Some('ґ'),  // 164-169
        Some('ء'),    Some('آ'),  Some('أ'),  Some('ؤ'),  Some('إ'),  Some('ئ'),  // 170-175
        Some('ا'),    Some('ب'),  Some('ة'),  Some('ت'),  Some('ث'),  Some('ج'),  // 176-181
        Some('ح'),    Some('خ'),  Some('د'),  Some('ذ'),  Some('ر'),  Some('ز'),  // 182-187
        Some('س'),    Some('ش'),  Some('ص'),  Some('ض'),  Some('ط'),  Some('ظ'),  // 188-193
        Some('ع'),    Some('غ'),  Some('ف'),  Some('ق'),  Some('ك'),  Some('ل'),  // 194-199
        Some('م'),    Some('ن'),  Some('ه'),  Some('و'),  Some('ى'),  Some('ي'),  // 200-205
        Some('ა'),    Some('ბ'),  Some('გ'),  Some('დ'),  Some('ე'),  Some('ვ'),  // 206-211
        Some('ზ'),    Some('თ'),  Some('ი'),  Some('კ'),  Some('ლ'),  Some('მ'),  // 212-217
        Some('ნ'),    Some('ო'),  Some('პ'),  Some('ჟ'),  Some('რ'),  Some('ს'),  // 218-223
        Some('ტ'),    Some('უ'),  Some('ფ'),  Some('ქ'),  Some('ღ'),  Some('ყ'),  // 224-229
        Some('შ'),    Some('ჩ'),  Some('ც'),  Some('ძ'),  Some('წ'),  Some('ჭ'),  // 230-235
        Some('ხ'),    Some('ჯ'),  Some('ჰ'),  Some('ჱ'),  Some('ჲ'),  Some('ჳ'),  // 236-241
        Some('ჴ'),    Some('ჵ'),  Some('ჶ'),  Some('ჷ'),  Some('ჸ'),  Some('ჹ'),  // 242-247
        Some('ჺ'),                                                                 // 248
        None, None, None, None, None, None, None,                                 // 249-255
    ];
    T[slot as usize]
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::format;

    use super::*;

    #[test]
    fn roundtrip_ascii() {
        for ch in 'a'..='z' {
            let slot = encode(ch).expect("encode failed");
            assert_eq!(decode(slot), Some(ch));
        }
    }

    #[test]
    fn roundtrip_extended() {
        for ch in ['é', 'ñ', 'ü', 'α', 'ω', 'а', 'я', 'ё', 'ا', 'ბ'] {
            let slot = encode(ch).expect(&format!("encode failed for {ch:?}"));
            assert_eq!(decode(slot), Some(ch));
        }
    }

    #[test]
    fn normalise_uppercase() {
        assert_eq!(encode_normalised('A'), encode('a'));
        assert_eq!(encode_normalised('É'), encode('é'));
        assert_eq!(encode_normalised('Ω'), encode('ω'));
        assert_eq!(encode_normalised('Я'), encode('я'));
    }

    #[test]
    fn sentinel_is_zero() {
        assert_eq!(decode(0), None);
    }
}
