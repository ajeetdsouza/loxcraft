import React from 'react';
import AceEditor from 'react-ace';

/**
 * Interface defines props for {@link Editor} component
 */
interface EditorProps {
  text: string;
  // eslint-disable-next-line no-unused-vars
  onChange: (value: string) => void;
}

/**
 * A code editor component for lox playground
 * @param props - Props for code editor 
 * @returns An editor component
 */
const Editor = ({ onChange, text }: EditorProps) =>
(
  <AceEditor
    className="h-100 font-monospace fs-6"
    focus
    mode="text"
    name="editor"
    onChange={onChange}
    showPrintMargin={false}
    value={text}
  />)


export { Editor, EditorProps };
