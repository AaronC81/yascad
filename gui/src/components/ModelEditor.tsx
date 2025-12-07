import { Editor, Monaco } from "@monaco-editor/react";
import { ComponentProps, useCallback, useEffect, useRef, useState } from "react";
import { editor } from "monaco-editor";
import yascadTokenizer from "./../monarchTokenizer";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { getCurrentWindow } from "@tauri-apps/api/window";
import useKeyboardShortcut from "../hooks/useKeyboardShortcut";

export default function ModelEditor({ onChange, onReset, ...props }: {
  onChange: (editor: editor.IStandaloneCodeEditor) => any,
  onReset: () => any,
} & Omit<ComponentProps<"div">, "onChange">) {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  function editorDidMount(editor: editor.IStandaloneCodeEditor, _: Monaco) {
    editorRef.current = editor;
  }

  function editorWillMount(monaco: Monaco) {
    monaco.languages.register({ id: "yascad" });
    monaco.languages.setMonarchTokensProvider("yascad", yascadTokenizer as any);
  }

  const [unsavedChanges, setUnsavedChanges] = useState(false);
  function editorChange(_1: any, _: any) {
    setUnsavedChanges(true);
    onChange(editorRef.current!);
  }

  const [currentPath, setCurrentPath] = useState<string | null>(null);

  // Update window state based on the state of the editor.
  // It's a bit cheeky to do this within the `ModelEditor` component, but the interface for this
  // editor would be a bit weird if we lifted state out.
  useEffect(() => {
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

  const confirmLosingUnsaved = useCallback(async () => {
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
  }, [unsavedChanges]);

  const newModel = useCallback(async () => {
    if (await confirmLosingUnsaved()) {
      editorRef.current!.setValue("");
      setUnsavedChanges(false);
      onReset();
    }
  }, [confirmLosingUnsaved, onReset]);

  const openModel = useCallback(async () => {
    if (await confirmLosingUnsaved()) {
      const file = await open({ multiple: false, directory: false });
      if (!file) {
        return;
      }

      const fileContent = await readTextFile(file);
      editorRef.current!.setValue(fileContent);

      setCurrentPath(file);
      setUnsavedChanges(false);
      onReset();
    }
  }, [confirmLosingUnsaved, onReset]);

  const saveModel = useCallback(async () => {
    if (!currentPath) {
      return saveModelAs();
    }

    const content = editorRef.current!.getValue();
    await writeTextFile(currentPath, content);

    setUnsavedChanges(false);
  }, [currentPath]);

  const saveModelAs = useCallback(async () => {
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
  }, []);

  useKeyboardShortcut({ ctrlCmd: true, key: "s" }, saveModel, [saveModel]);
  useKeyboardShortcut({ ctrlCmd: true, key: "n" }, newModel, [newModel]);
  useKeyboardShortcut({ ctrlCmd: true, key: "o" }, openModel, [openModel]);

  const { className, ...restProps } = props;
  return <div className={`flex flex-col ${className}`} {...restProps}>
    <div className="p-[5px] flex flex-row gap-[5px]">
      <button onClick={newModel}>New</button>
      <button onClick={openModel}>Open...</button>
      <button onClick={saveModel}>Save</button>
      <button onClick={saveModelAs}>Save As...</button>
    </div>
    <div className="flex-1">
      <Editor
        theme="vs-dark"
        language="yascad"
        beforeMount={editorWillMount}
        onMount={editorDidMount}
        onChange={editorChange}
      />
    </div>
  </div>
}
