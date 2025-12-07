import { Editor, Monaco } from "@monaco-editor/react";
import { editor } from "monaco-editor";
import yascadTokenizer from "./monarchTokenizer";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

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
  function editorChange(_1: any, _: any) {
    setStlDirty(true);  
  }

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

    // TODO: ideally the STL viewer we use can accept text instead
    // const stlDataUri = `data:text/plain;base64,${btoa(lastStl)}`;

    // TODO: Improve viewer:
    //   - Should start at more isometric angle
    //   - Perspective/ortho toggle
    //   - Show a grid
    // if (!stlViewerHandleRef.current) {
    //   console.log("Creating STL viewer");
    //   const klass = (window as any).StlViewer; // Library is loaded externally
    //   stlViewerHandleRef.current = new klass(stlViewerDomRef.current!, { models: [] });
    // } else {
    //   console.log("Cleaning up existing STL viewer");
    //   stlViewerHandleRef.current.remove_model(1);
    //   stlViewerHandleRef.current.clean();
    // }

    // stlViewerHandleRef.current.add_model({ id:1, filename: stlDataUri });
  };

  useEffect(() => {
    const listener = function (event: KeyboardEvent) {
      if (event.key === "F5") {
        renderPreview();
      }
    };
    document.addEventListener("keydown", listener);
    
    return () => document.removeEventListener("keydown", listener);
  }, [])

  return (
    <main className="flex flex-row h-screen">
      <div className="flex-1 flex flex-col">
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
        <div id="output-model" className="flex-1 overflow-auto font-mono text-left whitespace-break-spaces">
          {lastStl}
        </div>
        <div id="output-messages" className="flex-1 font-mono text-left whitespace-break-spaces">
          {stlError || "Build messages will be shown here."}
        </div>
      </div>
    </main>
  )
}

export default App;
