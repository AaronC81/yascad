import { invoke } from "@tauri-apps/api/core";
import { save } from '@tauri-apps/plugin-dialog';
import { create } from '@tauri-apps/plugin-fs';

import * as monaco from "monaco-editor";
import monarchTokenizer from "./monarchTokenizer";

let stlViewer: any;
let codeEditor: monaco.editor.IStandaloneCodeEditor;

let lastStl: string;

// Has the STL source been edited since a re-render?
// Starts as true because nothing was rendered yet
let stlDirty: boolean;
function setStlDirty(dirty: boolean) {
  stlDirty = dirty;

  const button = document.getElementById("export-stl-button")! as HTMLButtonElement;
  button.disabled = stlDirty;
}
setStlDirty(true);


async function renderPreview() {
  const code = codeEditor.getValue();

  const outputMessages = document.getElementById("output-messages")!;
  const outputModel = document.getElementById("output-model")!;

  try {
    lastStl = await invoke("render_preview", { code });
    document.getElementById("output-messages")!.innerText = `Rendered successfully`;
    setStlDirty(false);

    // TODO: ideally the STL viewer we use can accept text instead
    const stlDataUri = `data:text/plain;base64,${btoa(lastStl)}`;

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

async function exportStl() {
  // Should not be able to happen because the button gets disabled
  if (stlDirty) {
    alert("Cannot export dirty STL.")
    return;
  }

  const path = await save({
    filters: [{ name: "STL", extensions: ["stl"] }],
  });
  console.log(path);
  if (!path) {
    return;
  }

  const file = await create(path);
  await file.write(new TextEncoder().encode(lastStl));
  await file.close();
}


window.addEventListener("DOMContentLoaded", () => {
  document.getElementById("render-button")!.onclick = () => {
    renderPreview();
  };

  document.getElementById("export-stl-button")!.onclick = () => {
    exportStl();
  };
  
  monaco.languages.register({ id: "yascad" });
  monaco.languages.setMonarchTokensProvider("yascad", monarchTokenizer as any);

  codeEditor = monaco.editor.create(document.getElementById("code-input")!, {
    theme: "vs-dark",
    language: "yascad",
    automaticLayout: true,
    minimap: undefined,
  });

  codeEditor.getModel()?.onDidChangeContent(() => {
    setStlDirty(true);
  });

  document.addEventListener("keydown", function (event) {
    if (event.key === "F5") {
      renderPreview();
    }
  });
});
