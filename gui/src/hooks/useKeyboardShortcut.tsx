import { useCallback, useEffect } from "react";

export default function useKeyboardShortcut(
  options: {
    // The name of the key which triggers this shortcut.
    key: string,

    // Whether CTRL or CMD needs to be held (OS dependent) for the shortcut to trigger.
    ctrlCmd?: boolean,
  },
  action: () => any,
  deps: any[],
) {
  const cachedAction = useCallback(action, deps);

  useEffect(() => {
    const listener = function (event: KeyboardEvent) {
      const ctrlCmdPressed = event.ctrlKey || event.metaKey;

      if (event.key === options.key && (options.ctrlCmd ? ctrlCmdPressed : true)) {
        event.preventDefault();
        action();
      }
    };

    document.addEventListener("keydown", listener);
    return () => document.removeEventListener("keydown", listener);
  }, [cachedAction, ...deps]);
}
