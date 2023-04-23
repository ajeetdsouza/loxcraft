import React from 'react';

/**
 * Interface defining props for code output
 */
interface OutputProps {
  text: string;
};

/**
 * Component for displaying code execution output
 * @param props - Props for code output section
 * @returns An output section
 */
const Output = ({ text }: OutputProps) =>
(
  <pre
    className="h-100 font-monospace fs-6 ms-1"
    // eslint-disable-next-line react/no-danger
    dangerouslySetInnerHTML={{ __html: text }}
    id="output"
  />
);


export { Output, OutputProps };
