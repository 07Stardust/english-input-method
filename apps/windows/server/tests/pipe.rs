//! 真命名管道的端到端测试（仅 Windows）：监听线程 + 文件句柄客户端，走完「开会话 → 敲 nihao → 收候选」。

#![cfg(windows)]

use std::fs::OpenOptions;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use qingjian_core::Language;
use qingjian_platform::protocol::{
    ClientMessage, Hello, KeyEvent, PROTOCOL_VERSION, ServerMessage, SessionId, Welcome,
};
use qingjian_windows_server::ipc::pipe::serve_pipe;
use qingjian_windows_server::ipc::{read_message, write_message};
use qingjian_windows_server::{AssemblySpec, Router, RouterConfig, assembly};

const SESSION: SessionId = SessionId(1);

fn letter(c: char) -> KeyEvent {
    KeyEvent::new(c.to_ascii_uppercase() as u32, Some(c), Default::default())
}

/// 客户端重试打开管道，直到监听线程建好实例。
fn connect(name: &str) -> std::fs::File {
    for _ in 0..50 {
        match OpenOptions::new().read(true).write(true).open(name) {
            Ok(file) => return file,
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }
    panic!("连不上管道 {name}");
}

#[test]
fn named_pipe_round_trips_the_open_type_loop() {
    let name = format!(r"\\.\pipe\qingjian-test-{}", std::process::id());

    // 监听线程服务完一个客户端后阻塞等下一个，随进程退出即可。
    let server_name = name.clone();
    thread::spawn(move || {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let engine = assembly::assemble(&AssemblySpec {
            glossary: Some((
                Language::English,
                root.join("assets/sample/glossary-en.tsv"),
            )),
            ..AssemblySpec::new(root.join("assets/sample/dict.tsv"))
        })
        .expect("assemble engine from sample data");
        let mut router = Router::new(engine, RouterConfig::default());
        let (work_tx, work_rx) = std::sync::mpsc::channel();
        serve_pipe(&server_name, &mut router, work_tx, work_rx).expect("listen to named pipe");
    });

    let mut client = connect(&name);
    use std::os::windows::io::AsRawHandle;
    qingjian_platform::windows_security::verify_peer(client.as_raw_handle() as isize, false)
        .unwrap();
    write_message(
        &mut client,
        &Hello {
            protocol: PROTOCOL_VERSION,
        },
    )
    .unwrap();
    let _: Welcome = read_message(&mut client).unwrap().unwrap();

    write_message(
        &mut client,
        &ClientMessage::OpenSession {
            session: SESSION,
            app: None,
            protocol: PROTOCOL_VERSION,
        },
    )
    .unwrap();
    let ServerMessage::SessionOpened { session, .. } = read_message(&mut client).unwrap().unwrap()
    else {
        panic!("session expected");
    };
    assert_ne!(session, SESSION);
    for c in "nihao".chars() {
        write_message(
            &mut client,
            &ClientMessage::Key {
                session,
                event: letter(c),
            },
        )
        .unwrap();
    }

    // 开会话先回一条 `SessionOpened`，之后五个按键各回一条 `KeyResult`。
    let mut last_frame = None;
    for _ in 0..5 {
        let message: ServerMessage = read_message(&mut client)
            .expect("read response")
            .expect("server closed early");
        if let ServerMessage::KeyResult { frame, .. } = message {
            last_frame = Some(frame);
        }
    }

    let frame = last_frame.expect("至少一条 KeyResult");
    let preedit: String = frame.preedit.iter().map(|s| s.text.as_str()).collect();
    assert_eq!(preedit, "ni'hao");
    let texts: Vec<&str> = frame
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    assert!(
        texts.contains(&"你好"),
        "候选里应有「你好」，实际：{texts:?}"
    );
    // 另一连接不能读取、修改隐私状态或关闭当前连接拥有的会话。
    let mut intruder = connect(&name);
    write_message(
        &mut intruder,
        &Hello {
            protocol: PROTOCOL_VERSION,
        },
    )
    .unwrap();
    let _: Welcome = read_message(&mut intruder).unwrap().unwrap();
    write_message(
        &mut intruder,
        &ClientMessage::OpenSession {
            session: SESSION,
            app: None,
            protocol: PROTOCOL_VERSION,
        },
    )
    .unwrap();
    let ServerMessage::SessionOpened { session: other, .. } =
        read_message(&mut intruder).unwrap().unwrap()
    else {
        panic!("session expected");
    };
    assert_ne!(session, other);
    write_message(&mut intruder, &ClientMessage::Poll { session }).unwrap();
    let mut reader =
        qingjian_platform::windows_security::PipeReader::new(&mut intruder, Duration::from_secs(2));
    assert!(!matches!(
        read_message::<_, ServerMessage>(&mut reader),
        Ok(Some(_))
    ));
    write_message(&mut client, &ClientMessage::Poll { session }).unwrap();
    assert!(
        read_message::<_, ServerMessage>(&mut client)
            .unwrap()
            .is_some()
    );
    let mut occupied = Vec::new();
    for _ in 1..qingjian_windows_server::ipc::pipe::MAX_CONNECTIONS {
        let mut connection = connect(&name);
        write_message(
            &mut connection,
            &Hello {
                protocol: PROTOCOL_VERSION,
            },
        )
        .unwrap();
        let _: Welcome = read_message(&mut connection).unwrap().unwrap();
        occupied.push(connection);
    }
    let mut overflow = connect(&name);
    let _ = write_message(
        &mut overflow,
        &Hello {
            protocol: PROTOCOL_VERSION,
        },
    );
    let mut reader =
        qingjian_platform::windows_security::PipeReader::new(&mut overflow, Duration::from_secs(2));
    assert!(!matches!(
        read_message::<_, Welcome>(&mut reader),
        Ok(Some(_))
    ));
    drop(occupied);
    thread::sleep(Duration::from_millis(100));
    let mut recovered = connect(&name);
    write_message(
        &mut recovered,
        &Hello {
            protocol: PROTOCOL_VERSION,
        },
    )
    .unwrap();
    let _: Welcome = read_message(&mut recovered).unwrap().unwrap();
}
