import { useCallback, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import RenderCanvas from "./components/RenderCanvas";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import useKeyboardShortcut from "./hooks/useKeyboardShortcut";
import { editor } from "monaco-editor";
import ModelEditor from "./components/ModelEditor";
import { PanelGroup, Panel, PanelResizeHandle } from "react-resizable-panels";

function App() {
  const [lastStl, setLastStl] = useState("");
  const [stlError, setStlError] = useState<string | null>(null);
  const [stlDirty, setStlDirty] = useState(true);

  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  function editorChange(editor: editor.IStandaloneCodeEditor) {
    if (!editorRef.current) {
      editorRef.current = editor;
    }

    setStlDirty(true);
  }  

  const resetModelEditorState = useCallback(() => {
    setLastStl("");
    setStlError(null);
    setStlDirty(true);
  }, []);

  const renderPreview = useCallback(async () => {
    const code = editorRef.current!.getValue();
    try {
      setLastStl(await invoke("render_preview", { code }));
    } catch (e) {
      setStlError(String(e));
      return;
    }
    setStlError(null);
    setStlDirty(false);
  }, []);

  const exportStl = useCallback(async () => {
    const file = await save({
      filters: [
        {
          name: "STL",
          extensions: ["stl"]
        },
      ],
    });
    if (!file) {
      return;
    }

    await writeTextFile(file, lastStl);
  }, [lastStl]);

  useKeyboardShortcut({ key: "F5" }, renderPreview, [renderPreview]);

  return (
    <main className="h-screen">
      <PanelGroup autoSaveId={"mainPanelGroup"} direction="horizontal" className="w-screen h-screen">
        <Panel className="flex flex-col" defaultSize={50}>
          <ModelEditor
            className="flex-1"
            onChange={editorChange}
            onReset={resetModelEditorState}
          />
          <div className="flex flex-row p-[5px] gap-[5px]">
            <button className="flex-2" onClick={renderPreview}>Render (F5)</button>
            <button onClick={exportStl} className="flex-1" disabled={stlDirty}>Export STL</button>
          </div>
        </Panel>

        <PanelResizeHandle />

        <Panel className="flex flex-col" defaultSize={50}>
          <div id="output-model" className="flex-1 min-h-0">
            {/* Important: the canvas must remain mounted all the time */}
            <RenderCanvas stl={lastStl} />
          </div>
          
          <div id="output-messages" className={"font-mono text-left whitespace-break-spaces " + (stlError ? "flex-1" : "hidden")}>
            {stlError || "Build messages will be shown here."}
          </div>
        </Panel>
      </PanelGroup>
    </main>
  )
}

export default App;
