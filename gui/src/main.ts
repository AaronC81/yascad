import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import * as monaco from "monaco-editor";

let stlViewer: any;
let codeEditor: monaco.editor.IStandaloneCodeEditor;

async function renderPreview() {
  const code = codeEditor.getValue();
  console.log(code);

  try {
    const stlPath: string = await invoke("render_preview", { code });
    document.getElementById("output-messages")!.innerText = `Rendered successfully to ${stlPath}`;

    const stlUri = convertFileSrc(stlPath);

    if (!stlViewer) {
      console.log("Creating STL viewer");
      const klass = (window as any).StlViewer;
      // TODO: this doesn't work for a second render
      stlViewer = new klass(document.getElementById("output-model"), { models: [] });
    } else {
      console.log("Cleaning up existing STL viewer");
      stlViewer.remove_model(1);
      stlViewer.clean();
    }

    // Append a random number to force reload
    stlViewer.add_model({ id:1, filename: stlUri + "?" + Math.random() });
  } catch (e) {
    console.log(e);
    document.getElementById("output-messages")!.innerText = String(e);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  document.getElementById("render-preview-button")!.onclick = () => {
    renderPreview();
  };
  
  codeEditor = monaco.editor.create(document.getElementById("code-input")!, {
    theme: "vs-dark",
    automaticLayout: true,
    minimap: undefined,
  });
});
