import * as path from "path";
import { ExtensionContext, window } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(ctx: ExtensionContext) {
  try {
    const exe =
      process.env.JSP_LSP_BIN ??
      path.join(ctx.extensionPath, "..", "target", "release", "jsp-lsp");

    const serverOptions: ServerOptions = {
      command: exe,
      args: ["--stdio"],
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
