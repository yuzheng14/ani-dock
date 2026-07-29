use std::sync::LazyLock;

use rand::seq::IndexedRandom;

use crate::constant::ORIGIN;

static CHARS_VEC: LazyLock<Vec<char>> = LazyLock::new(|| {
    let chars = "abcdefghijklmnopqrstuvwxyz0123456789";
    chars.chars().collect::<Vec<char>>()
});

pub fn random_string(len: u32) -> String {
    let mut string = String::new();
    let mut rng = rand::rng();

    for _ in 0..len {
        string.push(CHARS_VEC.choose(&mut rng).unwrap().to_owned());
    }

    string
}

pub fn get_referer(sn: u32) -> String {
    format!("{ORIGIN}/animeVideo.php?sn={sn}")
}

pub fn sanitize_path_segment(path: &str) -> String {
    path.replace("|", "｜")
        .replace("?", "？")
        .replace("*", "＊")
        .replace("<", "＜")
        .replace(">", "＞")
        .replace('"', "＂")
        .replace(":", "：")
        .replace("\\", "＼")
        .replace("/", "／")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn random_string_has_requested_length() {
        for len in [0, 1, 12, 100] {
            assert_eq!(random_string(len).len(), len as usize);
        }
    }

    #[test]
    fn random_string_only_contains_supported_characters() {
        let value = random_string(1_000);

        assert!(
            value
                .chars()
                .all(|character| CHARS_VEC.contains(&character))
        );
    }

    #[test]
    fn get_referer_builds_anime_video_url() {
        assert_eq!(
            get_referer(49780),
            format!("{ORIGIN}/animeVideo.php?sn=49780")
        );
    }

    #[test]
    fn sanitize_path_segment_replaces_unsupported_characters() {
        assert_eq!(
            sanitize_path_segment(r#"a|b?c*d<e>f"g:h\i/j"#),
            "a｜b？c＊d＜e＞f＂g：h＼i／j"
        );
    }

    #[test]
    fn sanitize_path_segment_preserves_supported_characters() {
        let path_segment = "劇場版 關於我轉生變成史萊姆這檔事 [電影].mp4";

        assert_eq!(sanitize_path_segment(path_segment), path_segment);
    }
}
