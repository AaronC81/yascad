import { Editor, Monaco } from "@monaco-editor/react";
import { ComponentProps, useCallback, useEffect, useEffectEvent, useRef, useState } from "react";
import { editor } from "monaco-editor";
import yascadTokenizer from "./../monarchTokenizer";
import useKeyboardShortcut from "../hooks/useKeyboardShortcut";
import { pickOpenFile, pickSaveFile } from "../lib/file-picker";

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

  const [currentHandle, setCurrentHandle] = useState<FileSystemFileHandle | null>(null);

  const confirmLosingUnsaved = useCallback(async () => {
    if (unsavedChanges) {
      return confirm("Your model has unsaved changes. Are you sure you want to discard them?");
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
      const handle = await pickOpenFile("yascad");
      if (!handle) return;

      const fileContent = await (await handle.getFile()).text();
      editorRef.current!.setValue(fileContent);

      setCurrentHandle(handle);
      setUnsavedChanges(false);
      onReset();
    }
  }, [confirmLosingUnsaved, onReset]);

  const saveModel = useCallback(async () => {
    if (!currentHandle) {
      return saveModelAs();
    }

    const content = editorRef.current!.getValue();
    
    const writable = await currentHandle.createWritable();
    await writable.write(content);
    await writable.close();

    setUnsavedChanges(false);
  }, [currentHandle]);

  const saveModelAs = useCallback(async () => {
    const content = editorRef.current!.getValue();
    const handle = await pickSaveFile("yascad");
    
    if (!handle) return;

    const writable = await handle.createWritable();
    await writable.write(content);
    await writable.close();

    setCurrentHandle(handle);
    setUnsavedChanges(false);
  }, []);

  useKeyboardShortcut({ ctrlCmd: true, key: "s" }, saveModel, [saveModel]);
  useKeyboardShortcut({ ctrlCmd: true, key: "n" }, newModel, [newModel]);
  useKeyboardShortcut({ ctrlCmd: true, key: "o" }, openModel, [openModel]);

  // Prevent closing if there are unsaved changes
  const handleBeforeUnload = useEffectEvent((e: Event) => {
    if (unsavedChanges) {
      e.preventDefault();
    }
  });
  useEffect(() => {
    const handler = (e: Event) => handleBeforeUnload(e);
    window.addEventListener("beforeunload", handler);

    return () => window.removeEventListener("beforeunload", handler);
  }, []);

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
