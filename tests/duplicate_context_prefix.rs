#![cfg(unix)]

use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn duplicate_prefix_warns_and_executes_only_the_subcommand() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "with-duplicate-prefix-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&temp_dir).expect("create temporary directory");

    let executable = temp_dir.join("fake-git");
    let captured_args = temp_dir.join("args.txt");
    fs::write(
        &executable,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$WITH_TEST_CAPTURE_ARGS\"\n",
    )
    .expect("write fake command");
    let mut permissions = fs::metadata(&executable)
        .expect("read fake command metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).expect("make fake command executable");

    let mut child = Command::new(env!("CARGO_BIN_EXE_with"))
        .arg("fake-git")
        .env(
            "PATH",
            format!(
                "{}:{}",
                temp_dir.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("WITH_TEST_CAPTURE_ARGS", &captured_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start with REPL");

    child
        .stdin
        .take()
        .expect("REPL stdin")
        .write_all(b"fake-git push\nquit\n")
        .expect("send commands to REPL");

    let output = child.wait_with_output().expect("wait for with REPL");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let args = fs::read_to_string(&captured_args).expect("fake command should run");

    assert!(
        output.status.success(),
        "REPL exited unsuccessfully: {stderr}"
    );
    assert!(
        stderr.contains("warning: duplicated context prefix \"fake-git\" ignored"),
        "missing duplicate-prefix warning in stderr: {stderr}"
    );
    assert_eq!(args, "push\n");

    fs::remove_dir_all(&temp_dir).expect("remove temporary directory");
}
