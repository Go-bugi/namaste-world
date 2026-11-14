/// 엔드포인트 파일 내용. 훅 스크립트가 `KEY=VALUE` 줄로 읽는다.
pub fn endpoint_file(port: u16, token: &str) -> String {
    format!("NAMASTE_PORT={port}\nNAMASTE_TOKEN={token}\n")
}

/// 토큰이 맞는지. 길이가 같으면 끝까지 비교해 비교 시간으로 토큰을 알아내지 못하게 한다.
pub fn token_matches(expected: &str, given: Option<&str>) -> bool {
    let Some(given) = given else {
        return false;
    };
    expected.len() == given.len()
        && expected
            .bytes()
            .zip(given.bytes())
            .fold(0u8, |diff, (a, b)| diff | (a ^ b))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr_hook_008_endpoint_file_has_port_and_token_lines() {
        assert_eq!(
            endpoint_file(48123, "abc123"),
            "NAMASTE_PORT=48123\nNAMASTE_TOKEN=abc123\n"
        );
    }

    #[test]
    fn fr_hook_001_matching_token_is_accepted() {
        assert!(token_matches("s3cret-token", Some("s3cret-token")));
    }

    #[test]
    fn fr_hook_001_missing_token_is_rejected() {
        assert!(!token_matches("s3cret-token", None));
    }

    #[test]
    fn fr_hook_001_wrong_token_is_rejected() {
        assert!(!token_matches("s3cret-token", Some("s3cret-tokem")));
        assert!(!token_matches("s3cret-token", Some("s3cret")));
        assert!(!token_matches("s3cret-token", Some("s3cret-token-extra")));
        assert!(!token_matches("s3cret-token", Some("")));
    }

    mod props {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn fr_hook_001_token_accepted_only_when_equal(expected in "[A-Za-z0-9]{1,48}", given in "[A-Za-z0-9]{0,48}") {
                prop_assert!(token_matches(&expected, Some(&expected)));
                prop_assert_eq!(token_matches(&expected, Some(&given)), expected == given);
            }

            #[test]
            fn fr_hook_008_endpoint_file_round_trips(port in any::<u16>(), token in "[A-Za-z0-9]{1,64}") {
                let file = endpoint_file(port, &token);
                let lines: Vec<&str> = file.lines().collect();
                let port_line = format!("NAMASTE_PORT={port}");
                let token_line = format!("NAMASTE_TOKEN={token}");
                prop_assert_eq!(lines, vec![port_line.as_str(), token_line.as_str()]);
            }
        }
    }
}
