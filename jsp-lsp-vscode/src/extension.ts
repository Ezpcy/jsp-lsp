import * as path from "path";
import { ExtensionContext, workspace, window, commands } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(ctx: ExtensionContext) {
  try {
    const config = workspace.getConfiguration("jspLsp");
    const jar = config.get<string>("javaLauncherJar");
    const cfg = config.get<string>("javaConfigDir");

    if (!jar || !cfg) {
      window
        .showErrorMessage(
          "JSP LSP: Java LSP is not correctly configured.",
          "Open Settings"
        )
        .then((selection) => {
          if (selection === "Open Settings") {
            commands.executeCommand(
              "workbench.action.openSettings",
              "vscode://settings/jspLsp"
            );
          }
        });
      return;
    }

    const exe =
      process.env.JSP_LSP_BIN ??
      path.join(ctx.extensionPath, "..", "target", "debug", "jsp-lsp");

    const serverOptions: ServerOptions = {
      command: exe,
      args: ["--stdio", "-p", jar!, "-c", cfg!],
      transport: TransportKind.stdio,
    };

    const clientOptions: LanguageClientOptions = {
      documentSelector: [{ scheme: "file", language: "jsp" }],
    };

    client = new LanguageClient(
      "jsp-lsp",
      "JSP Language Server",
      serverOptions,
      clientOptions
    );

    client.start();
  } catch (err) {
    window.showErrorMessage(
      `JSP LSP: Activation error - ${(err as Error).message}`
    );
  }
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
