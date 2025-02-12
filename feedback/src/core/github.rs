use actix_web::HttpResponse;

use log::error;
use octocrab::Octocrab;
use regex::Regex;

pub async fn post_feedback(
    github_token: String,
    title_category: &str,
    title: &str,
    description: &str,
    labels: Vec<String>,
) -> HttpResponse {
    let raw_title = format!("[{title_category}] {title}");
    let title = clean_feedback_data(&raw_title, 512);
    let description = clean_feedback_data(description, 1024 * 1024);

    if title.len() < 3 || description.len() < 10 {
        return HttpResponse::UnprocessableEntity()
            .content_type("text/plain")
            .body("Subject or body missing or too short");
    }

    let octocrab = Octocrab::builder().personal_token(github_token).build();
    if octocrab.is_err() {
        error!("Error creating issue: {:?}", octocrab);
        return HttpResponse::InternalServerError().body("Could not create Octocrab instance");
    }

    let resp = octocrab
        .unwrap()
        .issues("TUM-Dev", "navigatum")
        .create(title)
        .body(description)
        .labels(labels)
        .send()
        .await;

    return match resp {
        Ok(issue) => HttpResponse::Created()
            .content_type("text/plain")
            .body(issue.html_url.to_string()),
        Err(e) => {
            error!("Error creating issue: {:?}", e);
            HttpResponse::InternalServerError()
                .content_type("text/plain")
                .body("Failed create issue")
        }
    };
}

/// Remove all returns a string, which has
/// - all control characters removed
/// - is at most len characters long
/// - can be nicely formatted in markdown (just \n in md is not a linebreak)
fn clean_feedback_data(s: &str, len: usize) -> String {
    let s_clean = s
        .chars()
        .filter(|c| !c.is_control() || (c == &'\n'))
        .take(len)
        .collect::<String>();

    let re = Regex::new(r"[ \t]*\n").unwrap();
    re.replace_all(&s_clean, "  \n").to_string()
}

#[cfg(test)]
mod description_tests {
    use super::*;

    #[test]
    fn newlines_whitespace() {
        assert_eq!(
            clean_feedback_data("a\r\nb", 9),
            clean_feedback_data("a\nb", 9)
        );
        assert_eq!(clean_feedback_data("a\nb\nc", 9), "a  \nb  \nc");
        assert_eq!(clean_feedback_data("a\nb  \nc", 9), "a  \nb  \nc");
        assert_eq!(clean_feedback_data("a      \nb", 9), "a  \nb");
        assert_eq!(clean_feedback_data("a\n\nb", 9), "a  \n  \nb");
        assert_eq!(clean_feedback_data("a\n   b", 9), "a  \n   b");
    }
    #[test]
    fn truncate_len() {
        for i in 0..10 {
            let mut expected = "abcd".to_string();
            expected.truncate(i);
            assert_eq!(clean_feedback_data("abcd", i), expected);
        }
    }
    #[test]
    fn special_cases() {
        assert_eq!(clean_feedback_data("", 0), "");
        assert_eq!(clean_feedback_data("a\x05bc", 9), "abc");
        assert_eq!(clean_feedback_data("ab\x0Dc", 9), "abc");
    }
}
