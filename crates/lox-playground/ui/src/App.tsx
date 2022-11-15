import React, { useState, useEffect } from "react";
import AceEditor from "react-ace";
import Split from "react-split";
// @ts-ignore
import Logo from "./assets/lox.png";

export function App(): JSX.Element {
  // Send resize signal to editor on initialization.
  // https://github.com/securingsincity/react-ace/issues/708
  useEffect(() => {
    window.dispatchEvent(new Event("resize"));
  }, []);

  // Editor text is saved to local storage.
  const [editorText, setEditorText] = useState<string>(LocalStorage.editorText);
  useEffect(() => {
    LocalStorage.editorText = editorText;
  }, [editorText]);

  // Output from Lox is continuously streamed here.
  const [outputText, setOutputText] = useState<string>("");
  const addOutputText = (text: string) => {
    setOutputText((outputText) => outputText + text);
  };

  // The worker is set back to null once it finishes executing.
  const [worker, setWorker] = useState<Worker | null>(null);
  const stopWorker = () => {
    setWorker((worker) => {
      if (worker !== null) {
        worker.terminate();
      }
      return null;
    });
  };

  const startLox = () => {
    stopWorker();
    setOutputText("");

    const worker = new Worker(new URL("worker.ts", import.meta.url), {
      type: "module",
    });
    worker.onmessage = (event) => {
      const msg: LoxOutMessage = JSON.parse(event.data);
      switch (msg.type) {
        case "Output":
          addOutputText(msg.text);
          break;
        case "ExitSuccess":
          stopWorker();
          addOutputText("---\nProgram exited successfully.\n");
          break;
        case "ExitFailure":
          stopWorker();
          addOutputText("---\nProgram exited with errors.\n");
          break;
      }
    };
    worker.postMessage(editorText);
    setWorker(worker);
  };

  const stopLox = () => {
    stopWorker();
    addOutputText("---\nCommand terminated.");
  };

  const isRunning = worker !== null;
  return (
    <>
      <Nav isRunning={isRunning} runClick={isRunning ? stopLox : startLox} />
      <Split
        className="d-flex"
        cursor="col-resize"
        direction="horizontal"
        id="content"
        // Send resize signal to editor on split resize.
        // https://github.com/securingsincity/react-ace/issues/708
        onDragEnd={() => window.dispatchEvent(new Event("resize"))}
      >
        <Editor text={editorText} onChange={setEditorText} />
        <Output text={outputText} />
      </Split>
    </>
  );
}

function Nav(props: NavProps): JSX.Element {
  let runColor = "btn-success";
  let runIcon = "me-1 bi bi-play-fill";
  let runText = "Run";
  if (props.isRunning) {
    runColor = "btn-danger";
    runIcon = "me-2 spinner-grow spinner-grow-sm";
    runText = "Stop";
  }

  return (
    <nav className="navbar p-2" id="navbar">
      <div className="navbar-brand">
        <img alt="Logo" className="me-2" src={Logo} />
        Loxcraft Playground
      </div>
      <div>
        <a
          className="btn btn-dark bi bi-github me-1"
          href="https://github.com/ajeetdsouza/loxcraft"
          target="_blank"
        />
        <a id="run-btn" className={`btn ${runColor}`} onClick={props.runClick}>
          <span className={runIcon} role="status" aria-hidden="true"></span>
          {runText}
        </a>
      </div>
    </nav>
  );
}

function Editor(props: EditorProps): JSX.Element {
  return (
    <AceEditor
      className="h-100 font-monospace fs-6"
      focus={true}
      mode="text"
      name="editor"
      onChange={props.onChange}
      showPrintMargin={false}
      value={props.text}
    />
  );
}

function Output(props: OutputProps): JSX.Element {
  return (
    <pre
      className="h-100 font-monospace fs-6 ms-1"
      dangerouslySetInnerHTML={{ __html: props.text }}
      id="output"
    />
  );
}

type NavProps = {
  isRunning: Boolean;
  runClick: () => void;
};

type EditorProps = {
  text: string;
  onChange: (text: string) => void;
};

type OutputProps = {
  text: string;
};

type LoxOutMessage =
  | LoxOutMessageOutput
  | LoxOutMessageExitFailure
  | LoxOutMessageExitSuccess;
type LoxOutMessageOutput = {
  type: "Output";
  text: string;
};
type LoxOutMessageExitFailure = {
  type: "ExitFailure";
};
type LoxOutMessageExitSuccess = {
  type: "ExitSuccess";
};

class LocalStorage {
  static #editorTextKey = "editorText";

  static get editorText(): string {
    return localStorage.getItem(this.#editorTextKey) || "";
  }

  static set editorText(text: string) {
    localStorage.setItem(this.#editorTextKey, text);
  }
}
