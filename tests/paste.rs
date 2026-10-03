mod support;
use std::{path::PathBuf, sync::Arc};
use support::runner::FakeRunner;
use tmux_manager::{
    prompts::paste::{PasteOutcome, PasteService, PasteTarget},
    tmux::{PaneId, TmuxClient},
};
const READY: &str = "%12\t0\t0\t0\t0\t1\t2004\twork/0:code.1\n";
#[tokio::test]
async fn paste_transfers_bytes_on_stdin_to_only_the_explicit_pane() {
    let runner = Arc::new(FakeRunner::new(vec![
        (0, READY, ""),
        (0, "", ""),
        (0, READY, ""),
        (0, "", ""),
        (0, "", ""),
    ]));
    let socket = Some(PathBuf::from("/tmp/test socket"));
    let service = PasteService::new(TmuxClient::with_runner(socket.clone(), runner.clone()));
    let body = "中文🙂\n'quoted' $(never-execute)\n";
    let target = PasteTarget {
        socket: socket.clone(),
        pane_id: PaneId::parse("%12").unwrap(),
    };
    assert!(matches!(
        service.paste(&target, body).await.unwrap(),
        PasteOutcome::Delivered { .. }
    ));
    let calls = runner.calls.lock().unwrap();
    assert!(calls.iter().all(|c| c.socket == socket));
    assert_eq!(calls[1].stdin, body.as_bytes());
    assert_eq!(calls[1].args[0], "load-buffer");
    assert!(calls[1].args[2].starts_with("tmux-manager-"));
    assert_eq!(
        calls[3].args,
        [
            "paste-buffer",
            "-p",
            "-r",
            "-b",
            &calls[1].args[2],
            "-t",
            "%12"
        ]
    );
    assert_eq!(calls[4].args, ["delete-buffer", "-b", &calls[1].args[2]]);
    assert!(calls.iter().all(|c| {
        !c.args
            .iter()
            .any(|a| a == "Enter" || a == "C-m" || a.contains(body))
    }));
}
#[tokio::test]
async fn unknown_or_disabled_bracketed_paste_only_buffers_multiline() {
    for state in [
        "%12\t0\t0\t0\t0\t\t\twork\n",
        "%12\t0\t0\t0\t0\t0\t25\twork\n",
    ] {
        let runner = Arc::new(FakeRunner::new(vec![(0, state, ""), (0, "", "")]));
        let outcome = PasteService::new(TmuxClient::with_runner(None, runner.clone()))
            .paste(
                &PasteTarget {
                    socket: None,
                    pane_id: PaneId::parse("%12").unwrap(),
                },
                "one\ntwo\n",
            )
            .await
            .unwrap();
        assert!(matches!(outcome, PasteOutcome::BufferedOnly { .. }));
        assert_eq!(runner.calls.lock().unwrap().len(), 2);
    }
}
#[tokio::test]
async fn unsafe_panes_and_wrong_socket_stop_before_loading() {
    for state in [
        "%12\t1\t0\t0\t0\t1\t2004\twork\n",
        "%12\t0\t1\t0\t0\t1\t2004\twork\n",
        "%12\t0\t0\t1\t0\t1\t2004\twork\n",
        "%13\t0\t0\t0\t0\t1\t2004\twork\n",
    ] {
        let runner = Arc::new(FakeRunner::new(vec![(0, state, "")]));
        assert!(
            PasteService::new(TmuxClient::with_runner(None, runner.clone()))
                .paste(
                    &PasteTarget {
                        socket: None,
                        pane_id: PaneId::parse("%12").unwrap()
                    },
                    "body"
                )
                .await
                .is_err()
        );
        assert_eq!(runner.calls.lock().unwrap().len(), 1);
    }
    let runner = Arc::new(FakeRunner::new(vec![]));
    assert!(
        PasteService::new(TmuxClient::with_runner(None, runner.clone()))
            .paste(
                &PasteTarget {
                    socket: Some("other".into()),
                    pane_id: PaneId::parse("%12").unwrap()
                },
                "body"
            )
            .await
            .is_err()
    );
    assert!(runner.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn failed_paste_keeps_unique_buffer_and_does_not_retry() {
    let runner = Arc::new(FakeRunner::new(vec![
        (0, READY, ""),
        (0, "", ""),
        (0, READY, ""),
        (1, "", "pane disappeared"),
    ]));
    let error = PasteService::new(TmuxClient::with_runner(None, runner.clone()))
        .paste(
            &PasteTarget {
                socket: None,
                pane_id: PaneId::parse("%12").unwrap(),
            },
            "body",
        )
        .await
        .unwrap_err()
        .to_string();
    let calls = runner.calls.lock().unwrap();
    assert_eq!(calls.len(), 4);
    assert!(error.contains(&calls[1].args[2]));
}

#[test]
fn popup_stays_open_to_show_a_delivered_paste_warning() {
    use tmux_manager::prompts::runtime::after_paste;
    let warning = "buffer tmux-manager-x 清除失敗；勿重貼".to_string();
    assert_eq!(
        after_paste(Ok(PasteOutcome::Delivered {
            warning: Some(warning.clone())
        })),
        Some(warning)
    );
    assert_eq!(
        after_paste(Ok(PasteOutcome::Delivered { warning: None })),
        None
    );
}
