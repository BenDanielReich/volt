const { spawn } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");
const vscode = require("vscode");

function activate(context) {
  const diags = vscode.languages.createDiagnosticCollection("volt");
  context.subscriptions.push(diags);

  const check = (doc) => {
    if (!doc || doc.languageId !== "volt") return;
    const voltc = findVoltc();
    if (!voltc) {
      vscode.window.showWarningMessage(
        "voltc not found. Build with `cargo build` or set volt.path."
      );
      return;
    }
    const tmp = path.join(os.tmpdir(), "volt-ide-" + Date.now() + ".volt");
    fs.writeFileSync(tmp, doc.getText());
    const folder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    const args = ["check", tmp];
    if (folder) args.push("--std-path", path.join(folder, "std"));
    const proc = spawn(voltc, args, {
      cwd: folder,
      windowsHide: true,
      shell: false,
    });
    let err = "";
    proc.stderr.on("data", (d) => {
      err += d.toString();
    });
    proc.on("error", (e) => {
      vscode.window.showErrorMessage("voltc: " + e.message);
    });
    proc.on("close", () => {
      try {
        fs.unlinkSync(tmp);
      } catch (_) {}
      diags.set(doc.uri, parseCheck(err));
    });
  };

  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(check),
    vscode.workspace.onDidOpenTextDocument(check),
    vscode.commands.registerCommand("volt.openIde", () => {
      const voltc = findVoltc();
      if (!voltc) {
        vscode.window.showErrorMessage("voltc not found. Run cargo build first.");
        return;
      }
      const cwd = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
      spawn(voltc, ["ide"], {
        cwd,
        detached: true,
        stdio: "ignore",
        windowsHide: false,
        shell: false,
      }).unref();
      vscode.window.showInformationMessage("Starting Volt IDE in your browser…");
    })
  );

  if (vscode.window.activeTextEditor) {
    check(vscode.window.activeTextEditor.document);
  }
}

function findVoltc() {
  const configured = vscode.workspace.getConfiguration("volt").get("path");
  if (configured && fs.existsSync(configured)) return configured;
  const folder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  const names =
    process.platform === "win32" ? ["voltc.exe", "voltc"] : ["voltc"];
  const extras = [];
  if (folder) {
    const targetRoot = path.join(folder, "target");
    for (const name of names) {
      extras.push(
        path.join(targetRoot, "debug", name),
        path.join(targetRoot, "release", name)
      );
    }
    try {
      for (const ent of fs.readdirSync(targetRoot, { withFileTypes: true })) {
        if (!ent.isDirectory()) continue;
        for (const name of names) {
          extras.push(
            path.join(targetRoot, ent.name, "debug", name),
            path.join(targetRoot, ent.name, "release", name)
          );
        }
      }
    } catch (_) {}
  }
  for (const p of extras) {
    if (fs.existsSync(p)) return p;
  }
  return configured || names[0];
}

function parseCheck(stderr) {
  const items = [];
  const blocks = stderr.split(/^(?=error: |warning: )/m);
  for (const block of blocks) {
    const head = block.match(/^(error|warning): (.+)$/m);
    if (!head) continue;
    const sev =
      head[1] === "warning"
        ? vscode.DiagnosticSeverity.Warning
        : vscode.DiagnosticSeverity.Error;
    const loc = block.match(/--> .*:(\d+):(\d+)/);
    const line = loc ? Math.max(0, parseInt(loc[1], 10) - 1) : 0;
    const col = loc ? Math.max(0, parseInt(loc[2], 10) - 1) : 0;
    items.push(
      new vscode.Diagnostic(new vscode.Range(line, col, line, col + 1), head[2], sev)
    );
  }
  return items;
}

function deactivate() {}

module.exports = { activate, deactivate };
