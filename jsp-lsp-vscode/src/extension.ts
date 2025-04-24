import * as path from "path";
import { ExtensionContext } from "vscode";
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(ctx: ExtensionContext) {
    const exe =
        process.env.JSP_LSP_BIN ??
        path.join(ctx.extensionPath, "..", "..", "target", "debug", "jsp-lsp");

    const serverOptions: ServerOptions = {
        command: exe,
        args: ["--stdio"],
        transport: TransportKind.stdio,
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: "file", language: "jsp" }],
    };

    client = new LanguageClient("jsp-lsp", "JSP Language Server", serverOptions, clientOptions);
    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
