use crate::interfaces::http::add_log;
use std::fmt::Write;
use std::sync::Arc;

use super::constants::NL_RESPONSE_TIMEOUT_SECS;
use crate::application::use_cases::template_matcher::TemplateMatch;

pub fn detect_indonesian(query: &str) -> bool {
    let indonesian_keywords = [
        "tampilkan",
        "cari",
        "semua",
        "data",
        "yang",
        "dengan",
        "dari",
        "adalah",
        "berapa",
        "jumlah",
        "daftar",
        "user",
        "pengguna",
        "alamat",
        "nama",
        "id",
        "filter",
        "berdasarkan",
        "urutkan",
        "terbesar",
        "terkecil",
    ];

    let query_lower = query.to_lowercase();
    indonesian_keywords
        .iter()
        .any(|&keyword| query_lower.contains(keyword))
}

pub fn generate_fallback_response(results_context: &str, is_indonesian: bool) -> String {
    if results_context.contains("No results found") || results_context.is_empty() {
        if is_indonesian {
            "Tidak ada hasil yang ditemukan untuk query Anda.".to_string()
        } else {
            "No results found for your query.".to_string()
        }
    } else if is_indonesian {
        format!(
            "Berikut hasil query Anda:\n\n{}\n\n(Silakan lihat bagian sumber untuk detail lengkap)",
            results_context
        )
    } else {
        format!(
            "Here are your query results:\n\n{}\n\n(Please see the source section for detailed results)",
            results_context
        )
    }
}

pub async fn generate_nl_response(
    llm_client: &Arc<dyn crate::infrastructure::llm_clients::LLMClient + Send + Sync>,
    config: &crate::domain::llm_config::LLMConfig,
    user_query: &str,
    results_context: &str,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    conversation_history: Option<&str>,
) -> String {
    let is_indonesian = detect_indonesian(user_query);
    let response_lang_instruction = if is_indonesian {
        "Respond in Indonesian (Bahasa Indonesia)."
    } else {
        "Respond in English."
    };

    let system_prompt = format!(
        r#"You are a friendly and helpful database assistant. {}
Your task is to present SQL query results in a CLEAR, STRUCTURED format.

## MANDATORY OUTPUT FORMAT

### For RESULTS FOUND (1+ rows):
**Line 1**: Single sentence summary (e.g., \"Found 5 users:\" or \"Nemu 5 data:\")
**Line 2**: Empty line
**Line 3+**: Markdown table with ALL results
**Last**: Optional one-line note

### For NO RESULTS:
**Line 1**: Simple one-line message (e.g., \"No data found.\" or \"Gak ada data.\")

## TABLE FORMAT RULES
- MUST use markdown table format with | separators
- First row: column headers
- Second row: separator line with ---|
- Keep column names SHORT (use aliases if needed)
- Only show relevant columns (skip internal IDs unless asked)

## EXAMPLES

### Indonesian - Hasil Ditemukan:
```
Nemu 3 user dengan role admin:

| nama | email | role |
|------|-------|------|
| Budi | budi@mail.com | admin |
| Siti | siti@mail.com | admin |
| Andi | andi@mail.com | admin |
```

### English - Results Found:
```
Found 5 orders:

| order_id | total | status |
|----------|-------|--------|
| ORD001 | $150 | completed |
| ORD002 | $75 | pending |
| ORD003 | $200 | completed |
```

### Indonesian - Tidak Ada Hasil:
```
Hmm, gak ada data yang cocok dengan kriteria tersebut.
```

## CRITICAL RULES
1. NEVER write long paragraphs describing each row
2. NEVER repeat column names in sentences
3. ALWAYS use tables for 2+ results
4. Keep summary to ONE sentence only
5. Keep note to ONE line max (or skip it)
6. Format dates consistently (YYYY-MM-DD preferred)

## RESPONSE MUST FIT THIS PATTERN:
[Summary sentence]

| col1 | col2 | col3 |
|------|------|------|
| data | data | data |

[Optional note]

That's it. Be concise!"#,
        response_lang_instruction
    );

    let user_prompt = if let Some(history) = conversation_history {
        format!(
            r#"Previous conversation:
{}

User Query: {}

Query Results:
{}

Please provide a clear, natural language response to the user's latest question based on these results and the conversation context above."#,
            history,
            user_query.trim(),
            results_context
        )
    } else {
        format!(
            r#"User Query: {}

Query Results:
{}

Please provide a clear, natural language response to the user's question based on these results."#,
            user_query.trim(),
            results_context
        )
    };

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Generating NL response with LLM (is_indonesian: {}, timeout: {}s)",
            is_indonesian, NL_RESPONSE_TIMEOUT_SECS
        ),
    );

    use std::time::Duration;
    use tokio::time::timeout;

    let llm_result = timeout(
        Duration::from_secs(NL_RESPONSE_TIMEOUT_SECS),
        llm_client.generate(config, &system_prompt, &user_prompt),
    )
    .await;

    match llm_result {
        Ok(Ok(response)) => {
            let cleaned = response.trim().to_string();
            add_log(
                logs,
                "DEBUG",
                "SQL-RAG",
                &format!("LLM response generated: {} chars", cleaned.len()),
            );
            cleaned
        }
        Ok(Err(e)) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!("LLM response generation failed: {}, using fallback", e),
            );
            generate_fallback_response(results_context, is_indonesian)
        }
        Err(_) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!(
                    "LLM response generation timed out after {}s, using fallback",
                    NL_RESPONSE_TIMEOUT_SECS
                ),
            );
            generate_fallback_response(results_context, is_indonesian)
        }
    }
}

pub async fn generate_nl_response_with_few_shot(
    llm_client: &Arc<dyn crate::infrastructure::llm_clients::LLMClient + Send + Sync>,
    config: &crate::domain::llm_config::LLMConfig,
    user_query: &str,
    results_context: &str,
    few_shot_prompt: &str,
    logs: &Arc<std::sync::Mutex<Vec<crate::interfaces::http::LogEntry>>>,
    conversation_history: Option<&str>,
) -> String {
    let is_indonesian = detect_indonesian(user_query);
    let response_lang_instruction = if is_indonesian {
        "Respond in Indonesian (Bahasa Indonesia)."
    } else {
        "Respond in English."
    };

    let system_prompt = format!(
        r#"You are a friendly and helpful database assistant. {}
Your task is to present SQL query results in a CLEAR, STRUCTURED format.

## MANDATORY OUTPUT FORMAT

### For RESULTS FOUND (1+ rows):
**Line 1**: Single sentence summary
**Line 2**: Empty line
**Line 3+**: Markdown table with ALL results
**Last**: Optional one-line note

### For NO RESULTS:
**Line 1**: Simple one-line message

## CRITICAL RULES
1. NEVER write long paragraphs describing each row
2. ALWAYS use tables for 2+ results
3. Keep summary to ONE sentence only
4. Keep note to ONE line max (or skip it)
"#,
        response_lang_instruction
    );

    let user_prompt = if let Some(history) = conversation_history {
        format!(
            r#"Previous conversation:
{}

{}

User Query: {}

Query Results:
{}

Please provide a clear, natural language response to the user's latest question based on these results and the conversation context above."#,
            history,
            few_shot_prompt,
            user_query.trim(),
            results_context
        )
    } else {
        format!(
            r#"{}

User Query: {}

Query Results:
{}

Please provide a clear, natural language response to the user's question based on these results."#,
            few_shot_prompt,
            user_query.trim(),
            results_context
        )
    };

    add_log(
        logs,
        "DEBUG",
        "SQL-RAG",
        &format!(
            "Generating NL response with few-shot prompt (is_indonesian: {}, prompt: {} chars, timeout: {}s)",
            is_indonesian,
            few_shot_prompt.len(),
            NL_RESPONSE_TIMEOUT_SECS
        ),
    );

    use std::time::Duration;
    use tokio::time::timeout;

    let llm_result = timeout(
        Duration::from_secs(NL_RESPONSE_TIMEOUT_SECS),
        llm_client.generate(config, &system_prompt, &user_prompt),
    )
    .await;

    match llm_result {
        Ok(Ok(response)) => {
            let cleaned = response.trim().to_string();
            add_log(
                logs,
                "DEBUG",
                "SQL-RAG",
                &format!("Few-shot LLM response generated: {} chars", cleaned.len()),
            );
            add_log(
                logs,
                "DEBUG",
                "SQL-RAG",
                &format!(
                    "Response content preview: {}",
                    &cleaned[..cleaned.len().min(200)]
                ),
            );
            cleaned
        }
        Ok(Err(e)) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!(
                    "Few-shot LLM response generation failed: {}, using fallback",
                    e
                ),
            );
            generate_fallback_response(results_context, is_indonesian)
        }
        Err(_) => {
            add_log(
                logs,
                "WARN",
                "SQL-RAG",
                &format!(
                    "Few-shot LLM response generation timed out after {}s, using fallback",
                    NL_RESPONSE_TIMEOUT_SECS
                ),
            );
            generate_fallback_response(results_context, is_indonesian)
        }
    }
}

pub fn build_nl_few_shot_examples(user_query: &str, matched_templates: &[TemplateMatch]) -> String {
    let is_indonesian = detect_indonesian(user_query);
    let mut examples = String::from("## Example Response Format\n\n");

    if is_indonesian {
        examples.push_str(
            r#"### Contoh Format WAJIB

**Query**: Cari semua user dengan role admin

**Response BENAR**:
```
Nemu 3 user dengan role admin:

| nama | email | role |
|------|-------|------|
| Budi | budi@mail.com | admin |
| Siti | siti@mail.com | admin |
| Andi | andi@mail.com | admin |
```

**Response SALAH** (jangan seperti ini):
```
Oke, aku nemu 3 user dengan role admin. Ada user Budi dengan email budi@mail.com dan role admin...
```

"#,
        );
    } else {
        examples.push_str(
            r#"### Required Format

**Query**: Find all users with admin role

**CORRECT Response**:
```
Found 3 users with admin role:

| name | email | role |
|------|-------|------|
| John | john@mail.com | admin |
| Jane | jane@mail.com | admin |
| Bob | bob@mail.com | admin |
```

**WRONG Response** (don't do this):
```
I found 3 users with admin role. There's John with email...
```

"#,
        );
    }

    for (idx, template_match) in matched_templates.iter().take(2).enumerate() {
        let template = &template_match.template;
        if is_indonesian {
            writeln!(
                examples,
                "### Contoh {}\n**Pertanyaan:** {}\n**Format Jawaban:**\n- Summary 1 kalimat\n- Tabel markdown\n- Catatan singkat (opsional)\n",
                idx + 1,
                template.example_question
            )
            .unwrap();
        } else {
            writeln!(
                examples,
                "### Example {}\n**Question:** {}\n**Response Format:**\n- 1 sentence summary\n- Markdown table\n- Brief note (optional)\n",
                idx + 1,
                template.example_question
            )
            .unwrap();
        }
    }

    if is_indonesian {
        writeln!(
            examples,
            "\n### Format Wajib:\n1. **Line 1**: Summary 1 kalimat (cth: \"Nemu 3 data:\")\n2. **Line 2**: Kosong\n3. **Line 3+**: Tabel markdown\n4. **Terakhir**: Catatan 1 baris (opsional)\n\nJANGAN tulis narasi panjang. Gunakan tabel."
        )
        .unwrap();
    } else {
        writeln!(
            examples,
            "\n### Required Format:\n1. **Line 1**: 1 sentence summary (e.g. \"Found 3 records:\")\n2. **Line 2**: Empty\n3. **Line 3+**: Markdown table\n4. **Last**: 1-line note (optional)\n\nNO long narratives. Use tables."
        )
        .unwrap();
    }

    examples
}
