const TYPES = {
  stl: {
    description: "STL model",
    accept: { "model/stl": [".stl"] },
  },
  yascad: {
    description: "YASCAD code",
    accept: { "text/x-yascad": [".yascad"] },
  },
}

export type FileType = keyof typeof TYPES;

// TODO: APIs are Chrome-only... :(

export async function pickSaveFile(type: FileType): Promise<FileSystemFileHandle | undefined> {
  try {
    return await (window as any).showSaveFilePicker({
      excludeAcceptAllOption: true,
      types: [TYPES[type]],
    });
  } catch (e) {
    // Probably aborted
    return undefined;
  }
}

export async function pickOpenFile(type: FileType): Promise<FileSystemFileHandle | undefined> {
  try {
    return (await (window as any).showOpenFilePicker({
      excludeAcceptAllOption: true,
      types: [TYPES[type]],
    }))[0];
  } catch (e) {
    // Probably aborted
    return undefined;
  }
}