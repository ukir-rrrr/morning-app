use std::process::ExitCode;

use clap::Parser;
use morning_collector::{cli::Cli, run, EXIT_FAILURE, EXIT_OK};

#[tokio::main]
async fn main() -> ExitCode {
    // clap の既定の終了コード 2 は「認証失効」と衝突するため、引数エラーは 1 にする。
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let code = if err.use_stderr() {
                EXIT_FAILURE
            } else {
                EXIT_OK
            };
            let _ = err.print();
            return ExitCode::from(code);
        }
    };
    match run(cli).await {
        Ok(()) => ExitCode::from(EXIT_OK),
        Err(err) => {
            eprintln!("[morning-collector] error: {err}");
            ExitCode::from(err.exit_code())
        }
    }
}
