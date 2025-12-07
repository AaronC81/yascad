import { Editor, Monaco } from "@monaco-editor/react";
import { editor } from "monaco-editor";
import yascadTokenizer from "./monarchTokenizer";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import RenderCanvas from "./components/RenderCanvas";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { getCurrentWindow } from "@tauri-apps/api/window";

function App() {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  function editorDidMount(editor: editor.IStandaloneCodeEditor, _: Monaco) {
    editorRef.current = editor;
  }

  function editorWillMount(monaco: Monaco) {
    monaco.languages.register({ id: "yascad" });
    monaco.languages.setMonarchTokensProvider("yascad", yascadTokenizer as any);
  }

  const [lastStl, setLastStl] = useState("");
  const [stlError, setStlError] = useState<string | null>(null);
  
  const [stlDirty, setStlDirty] = useState(true);
  const [unsavedChanges, setUnsavedChanges] = useState(false);
  function editorChange(_1: any, _: any) {
    setStlDirty(true);  
    setUnsavedChanges(true);
  }

  const [currentPath, setCurrentPath] = useState<string | null>(null);
  useEffect(() => {
    console.log("effect trigger!");
    (async () => {
      const window = getCurrentWindow();
      const unsavedIndicator = unsavedChanges ? "*" : "";
      if (currentPath) {
        await window.setTitle(`${unsavedIndicator}${currentPath} - YASCAD`);
      } else {
        await window.setTitle(`${unsavedIndicator}Untitled - YASCAD`);
      }
    })();
  }, [currentPath, unsavedChanges]);

  const confirmLosingUnsaved = async () => {
    if (unsavedChanges) {
      // For some reason, Tauri is completely incompatible with the browser here, and `confirm`
      // returns a promise rather than a synchronous boolean.
      //
      // Even TypeScript thinks this is wrong, and I'm not surprised!
      //
      // The weird-looking cast gets us back on track.
      return await (confirm as unknown as (message?: string) => Promise<boolean>)("Your model has unsaved changes. Are you sure you want to discard them?");
    } else {
      // Changes are saved, don't need to warn
      return true;
    }
  };

  const resetModelEditorState = () => {
    setLastStl("");
    setStlError(null);
    setStlDirty(true);
    setUnsavedChanges(false);
    setCurrentPath(null);
  }

  const newModel = async () => {
    if (await confirmLosingUnsaved()) {
      resetModelEditorState();
      editorRef.current!.setValue("");

      setUnsavedChanges(false);
    }
  };

  const openModel = async () => {
    if (await confirmLosingUnsaved()) {
      const file = await open({ multiple: false, directory: false });
      if (!file) {
        return;
      }

      resetModelEditorState();

      const fileContent = await readTextFile(file);
      editorRef.current!.setValue(fileContent);

      setCurrentPath(file);
      setUnsavedChanges(false);
    }
  };

  const saveModel = async () => {
    if (!currentPath) {
      return saveModelAs();
    }

    const content = editorRef.current!.getValue();
    await writeTextFile(currentPath, content);

    setUnsavedChanges(false);
  };

  const saveModelAs = async () => {
    const file = await save({
      filters: [
        {
          name: "YASCAD Model",
          extensions: ["yascad"]
        },
      ]
    });
    if (!file) {
      return;
    }

    const content = editorRef.current!.getValue();
    await writeTextFile(file, content);

    setCurrentPath(file);
    setUnsavedChanges(false);
  };

  const renderPreview = async () => {
    const code = editorRef.current!.getValue();
    try {
      setLastStl(await invoke("render_preview", { code }));
    } catch (e) {
      setStlError(String(e));
      return;
    }
    setStlError(null);
    setStlDirty(false);
  };

  useEffect(() => {
    const listener = function (event: KeyboardEvent) {
      const ctrlOrCmd = event.ctrlKey || event.metaKey;

      if (event.key === "F5") {
        event.preventDefault();
        renderPreview();
      } else if (ctrlOrCmd && event.key == "s") {
        event.preventDefault();
        saveModel();
      } else if (ctrlOrCmd && event.key == "n") {
        newModel();
      } else if (ctrlOrCmd && event.key == "o") {
        openModel();
      }
    };
    document.addEventListener("keydown", listener);
    
    return () => document.removeEventListener("keydown", listener);
  }, [renderPreview, saveModel, newModel, openModel]);

  return (
    <main className="flex flex-row h-screen">
      <div className="flex-1 flex flex-col">
        <div className="p-[5px] flex flex-row gap-[5px]">
          <button onClick={newModel}>New</button>
          <button onClick={openModel}>Open...</button>
          <button onClick={saveModel}>Save</button>
          <button onClick={saveModelAs}>Save As...</button>
        </div>
        <div id="code-input" className="flex-1">
          <Editor
            theme="vs-dark"
            language="yascad"
            beforeMount={editorWillMount}
            onMount={editorDidMount}
            onChange={editorChange}
          />
        </div>
        <div className="flex flex-row p-[5px] gap-[5px]">
          <button id="render-button" className="flex-2" onClick={renderPreview}>Render (F5)</button>
          <button id="export-stl-button" className="flex-1" disabled={stlDirty}>Export STL</button>
        </div>
      </div>

      <div className="flex-1 p-[10px] flex flex-col">
        <div id="output-model" className="flex-1 min-h-0">
          {/* Important: the canvas must remain mounted all the time */}
          <RenderCanvas stl={lastStl} />
        </div>
        
        <div id="output-messages" className={"font-mono text-left whitespace-break-spaces " + (stlError ? "flex-1" : "hidden")}>
          {stlError || "Build messages will be shown here."}
        </div>
      </div>
    </main>
  )
}

export default App;
