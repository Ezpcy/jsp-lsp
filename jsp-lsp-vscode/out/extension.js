"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const path = __importStar(require("path"));
const vscode_1 = require("vscode");
const node_1 = require("vscode-languageclient/node");
let client;
function activate(ctx) {
    var _a;
    try {
        const config = vscode_1.workspace.getConfiguration("jspLsp");
        const jar = config.get("javaLauncherJar");
        const cfg = config.get("javaConfigDir");
        if (!jar || !cfg) {
            vscode_1.window
                .showErrorMessage("JSP LSP: Java LSP is not correctly configured.", "Open Settings")
                .then((selection) => {
                if (selection === "Open Settings") {
                    vscode_1.commands.executeCommand("workbench.action.openSettings", "vscode://settings/jspLsp");
                }
            });
            return;
        }
        const exe = (_a = process.env.JSP_LSP_BIN) !== null && _a !== void 0 ? _a : path.join(ctx.extensionPath, "..", "target", "debug", "jsp-lsp");
        const serverOptions = {
            command: exe,
            args: ["--stdio", "-p", jar, "-c", cfg],
            transport: node_1.TransportKind.stdio,
        };
        const clientOptions = {
            documentSelector: [{ scheme: "file", language: "jsp" }],
        };
        client = new node_1.LanguageClient("jsp-lsp", "JSP Language Server", serverOptions, clientOptions);
        client.start();
    }
    catch (err) {
        vscode_1.window.showErrorMessage(`JSP LSP: Activation error - ${err.message}`);
    }
}
function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
