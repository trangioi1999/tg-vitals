//! Runtime and model stores that must never be offered as cache junk.

/// Case-insensitive child-name prefixes. Tools run from these folders or load models
/// from them; "it looks like a cache" is not evidence that it is safe to delete.
pub const PROTECTED_CACHE_PREFIXES: &[&str] = &[
    "ms-playwright", "cypress", "camoufox", "puppeteer",
    "llama.cpp", "buzz", "huggingface", "whisper", "ollama",
];

pub fn is_protected_cache_child(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    PROTECTED_CACHE_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_runtime_and_model_prefixes_case_insensitively() {
        for name in [
            "ms-playwright", "ms-playwright-go", "ms-playwright-mcp", "MS-Playwright-MCP",
            "Cypress", "CYPRESS-browsers", "camoufox", "Camoufox-runtime",
            "puppeteer", "PUPPETEER-browser", "llama.cpp", "LLAMA.CPP-models",
            "Buzz", "BUZZ-models", "huggingface", "HuggingFace-hub",
            "whisper", "Whisper-models", "ollama", "OLLAMA-models",
        ] {
            assert!(is_protected_cache_child(name), "{name}");
        }
    }

    #[test]
    fn allows_unrelated_cache_names() {
        for name in ["", "com.app", "Homebrew", "pip", "ms-play", "llama", "com.ollama", "my-whisper"] {
            assert!(!is_protected_cache_child(name), "{name}");
        }
    }
}
