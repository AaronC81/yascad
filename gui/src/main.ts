import { invoke } from "@tauri-apps/api/core";
import * as monaco from "monaco-editor";
import monarchTokenizer from "./monarchTokenizer";

let stlViewer: any;
let codeEditor: monaco.editor.IStandaloneCodeEditor;

async function renderPreview() {
  const code = codeEditor.getValue();

  const outputMessages = document.getElementById("output-messages")!;
  const outputModel = document.getElementById("output-model")!;

  try {
    const stl: string = await invoke("render_preview", { code });
    document.getElementById("output-messages")!.innerText = `Rendered successfully`;

    // TODO: ideally the STL viewer we use can accept text instead
    const stlDataUri = `data:text/plain;base64,${btoa(stl)}`;

    // TODO: Improve viewer:
    //   - Should start at more isometric angle
    //   - Perspective/ortho toggle
    //   - Show a grid
    if (!stlViewer) {
      console.log("Creating STL viewer");
      const klass = (window as any).StlViewer;
      stlViewer = new klass(document.getElementById("output-model"), { models: [] });
    } else {
      console.log("Cleaning up existing STL viewer");
      stlViewer.remove_model(1);
      stlViewer.clean();
    }

    // Append a random number to force reload
    stlViewer.add_model({ id:1, filename: stlDataUri });

    outputModel.style.display = "block";
    outputMessages.style.display = "none";
  } catch (e) {
    outputMessages.innerText = String(e);

    outputModel.style.display = "none";
    outputMessages.style.display = "block";
  }
}


window.addEventListener("DOMContentLoaded", () => {
  document.getElementById("render-preview-button")!.onclick = () => {
    renderPreview();
  };
  
  monaco.languages.register({ id: "yascad" });
  monaco.languages.setMonarchTokensProvider("yascad", monarchTokenizer as any);

  codeEditor = monaco.editor.create(document.getElementById("code-input")!, {
    theme: "vs-dark",
    language: "yascad",
    automaticLayout: true,
    minimap: undefined,
  });

  document.addEventListener("keydown", function (event) {
    if (event.key === "F5") {
      renderPreview();
    }
  });
});
