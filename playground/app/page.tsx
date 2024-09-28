"use client";

import { Button } from "@/components/ui/button";
import AceEditor from "react-ace";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { Github, Loader2, Lock, Play } from "lucide-react";
import Link from "next/link";
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area";
import "ace-builds/src-noconflict/theme-tomorrow_night_bright";
import dynamic from "next/dynamic";
import React from "react";
import { create } from "zustand";
import { persist } from "zustand/middleware";

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

type LoxOutMessage =
  | LoxOutMessageOutput
  | LoxOutMessageExitFailure
  | LoxOutMessageExitSuccess;

type State = {
  editorText: string;
  outputText: string;
  worker?: Worker;
  workerStartTime: number;
};

type Action = {
  setEditorText: (text: string) => void;
  startVM: () => void;
  terminateVM: () => void;
  isVMRunning: () => boolean;
};

const useStore = create<State & Action>()(
  persist(
    (set, get) => ({
      editorText: "",
      outputText: "",
      worker: null,
      workerStartTime: 0,

      setEditorText: (text: string) => {
        set({ editorText: text });
      },

      startVM: () => {
        const worker = new Worker(new URL("worker.ts", import.meta.url), {
          type: "module",
        });
        worker.onmessage = (event) => {
          const msg = JSON.parse(event.data) as LoxOutMessage;
          switch (msg.type) {
            case "Output": {
              set((state) => ({ outputText: state.outputText + msg.text }));
              break;
            }
            case "ExitSuccess": {
              set((state) => {
                const elapsedTime = (Date.now() - state.workerStartTime) / 1000;
                const outputText = `${state.outputText}---\nProgram exited successfully (${elapsedTime}s).\n`;

                state.worker?.terminate();

                return {
                  outputText: outputText,
                  worker: null,
                  workerStartTime: 0,
                };
              });
              break;
            }
            case "ExitFailure": {
              set((state) => {
                const elapsedTime = (Date.now() - state.workerStartTime) / 1000;
                const outputText = `${state.outputText}---\nProgram exited with errors (${elapsedTime}s).\n`;

                state.worker?.terminate();

                return {
                  outputText: outputText,
                  worker: null,
                  workerStartTime: 0,
                };
              });
              break;
            }
          }
        };

        set({
          outputText: "",
          worker: worker,
          workerStartTime: Date.now(),
        });
        worker.postMessage(get().editorText);
      },

      terminateVM: () => {
        set((state) => {
          const elapsedTime = (Date.now() - state.workerStartTime) / 1000;
          const outputText = `${state.outputText}---\nProgram exited terminated (${elapsedTime}s).\n`;
          return {
            outputText: outputText,
            worker: null,
            workerStartTime: 0,
          };
        });
      },

      isVMRunning: () => get().worker !== null,
    }),
    {
      name: "loxcraft",
      partialize: (state) => ({
        editorText: state.editorText,
        outputText: state.outputText,
      }),
    },
  ),
);

function Page() {
  const {
    editorText,
    outputText,
    setEditorText,
    startVM,
    terminateVM,
    isVMRunning,
  } = useStore();
  const isRunning = isVMRunning();

  return (
    <div className="flex flex-col h-screen">
      <nav className="bg-background border-b flex items-center justify-between select-none p-4">
        <div>
          <Link href="/">
            <div className="flex font-mono font-semibold items-center text-lg">
              L<Lock className="size-3" strokeWidth={4}></Lock>
              xcraft Playground
            </div>
          </Link>
        </div>
        <div className="space-x-1">
          <Button
            className="min-w-28"
            variant={isRunning ? "destructive" : "default"}
            onClick={isRunning ? terminateVM : startVM}
          >
            {isRunning ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Cancel
              </>
            ) : (
              <>
                <Play className="mr-2 h-4 w-4" />
                Run
              </>
            )}
          </Button>

          <Button asChild variant="outline">
            <Link
              href="https://github.com/ajeetdsouza/loxcraft"
              target="_blank"
            >
              <Github className="h-4 w-4" />
            </Link>
          </Button>
        </div>
      </nav>

      <ResizablePanelGroup
        autoSaveId="splitSize"
        className="flex flex-grow"
        direction="horizontal"
      >
        <ResizablePanel className="h-full p-2 w-full">
          <AceEditor
            className="rounded-md"
            height="100%"
            focus
            mode={null}
            name="editor"
            onChange={setEditorText}
            setOptions={{
              cursorStyle: "slim",
            }}
            showPrintMargin={false}
            theme="tomorrow_night_bright"
            value={editorText}
            width="100%"
          />
        </ResizablePanel>
        <ResizableHandle />
        <ResizablePanel className="h-full p-2 w-full">
          <ScrollArea className="border h-full p-1 rounded-md w-full">
            <div
              className="font-mono text-sm whitespace-pre min-h-max min-w-max"
              dangerouslySetInnerHTML={{ __html: outputText }}
            ></div>
            <ScrollBar orientation="horizontal" />
          </ScrollArea>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}

export default dynamic(() => Promise.resolve(Page), {
  ssr: false,
});
