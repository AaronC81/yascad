import { useCallback, useRef, useState } from "react";
import RenderCanvas from "./components/RenderCanvas";
import useKeyboardShortcut from "./hooks/useKeyboardShortcut";
import { editor } from "monaco-editor";
import ModelEditor from "./components/ModelEditor";
import { PanelGroup, Panel, PanelResizeHandle } from "react-resizable-panels";
import { BuildStlOutput, buildYascadModelToStl } from "yascad-wasm";
import { pickSaveFile } from "./lib/file-picker";

function App() {
  const [lastBuildOutput, setLastBuildOutput] = useState<BuildStlOutput | null>(null);
  const [stlError, setStlError] = useState<string | null>(null);
  const [stlDirty, setStlDirty] = useState(true);

  const [currentOutputView, setCurrentOutputView] = useState("___main___");
  let currentStl = currentOutputView === "___main___"
    ? lastBuildOutput?.main
    : lastBuildOutput?.outputs?.[currentOutputView];

  // If the current output stops existing, revert to main
  if (
    lastBuildOutput
    && currentOutputView !== "___main___"
    && !Object.hasOwn(lastBuildOutput.outputs, currentOutputView)
  ) {
    setCurrentOutputView("___main___");
  }

  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  function editorChange(editor: editor.IStandaloneCodeEditor) {
    if (!editorRef.current) {
      editorRef.current = editor;
    }

    setStlDirty(true);
  }  

  const resetModelEditorState = useCallback(() => {
    setLastBuildOutput(null);
    setStlError(null);
    setStlDirty(true);
  }, []);

  const renderPreview = useCallback(async () => {
    const code = editorRef.current!.getValue();
    try {
      const stl = buildYascadModelToStl(code);
      setLastBuildOutput(stl);
    } catch (e) {
      setStlError(String(e));
      return;
    }
    setStlError(null);
    setStlDirty(false);
  }, []);

  const exportStl = useCallback(async () => {
    const handle = await pickSaveFile("stl");

    if (handle && lastBuildOutput) {
      const writable = await handle.createWritable();
      await writable.write(currentStl ?? "");
      await writable.close();
    }
  }, [lastBuildOutput]);

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
          {Object.keys(lastBuildOutput?.outputs ?? {}).length > 0 &&
            <div className="p-2">
              <select value={currentOutputView} onChange={e => setCurrentOutputView(e.target.value)}>
                <option value="___main___">Combined Output</option>
                {Object.keys(lastBuildOutput?.outputs ?? {})
                  .toSorted()
                  .map((output) =>
                    <option key={output} value={output}>{output}</option>
                  )
                }
              </select>
            </div>
          }

          <div id="output-model" className="flex-1 min-h-0">
            {/* Important: the canvas must remain mounted all the time */}
            <RenderCanvas stl={currentStl ?? ""} />
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
