use ai_runner::runners::claude::ClaudeRunner;
use ai_runner::{CliCfg, Runner, RunnerInput};
use tempfile::TempDir;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[tokio::test]
#[ignore = "requires a local authenticated Claude Code CLI and performs a live model run"]
async fn claude_cli_runner_can_create_text_file_in_temp_dir() {
    let tmp = TempDir::new().expect("temp dir");
    let target = tmp.path().join("claude_cli_edit_probe.txt");

    let prompt = "\
Create a file named `claude_cli_edit_probe.txt` in the current working directory.
The file must contain exactly:
rubix claude cli edit probe

Do not create or modify any other files. After writing the file, answer with only `done`.
";

    let runner = ClaudeRunner;
    let (tx, _rx) = mpsc::channel(16);
    let cancel = CancellationToken::new();
    let result = runner
        .run(
            RunnerInput::Cli(CliCfg {
                prompt: prompt.to_string(),
                work_dir: Some(tmp.path().to_string_lossy().into_owned()),
                allowed_tools: Some("Write".to_string()),
                ..Default::default()
            }),
            "claude-cli-edit-probe".into(),
            tx,
            cancel,
        )
        .await
        .expect("claude runner accepts cli input");

    assert!(
        result.error.is_none(),
        "claude run failed: {:?}",
        result.error
    );
    assert!(target.is_file(), "expected Claude CLI to create {target:?}");
    assert_eq!(
        std::fs::read_to_string(&target).expect("probe file contents"),
        "rubix claude cli edit probe\n"
    );
}
