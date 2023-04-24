import React from 'react';

interface OutputProps {
  text: string;
}

/**
 * Code execution output component
 */
const Output = ({ text }: OutputProps) => (
  <pre
    className="h-100 font-monospace fs-6 ms-1"
    // eslint-disable-next-line react/no-danger
    dangerouslySetInnerHTML={{ __html: text }}
    id="output"
  />
);

export { Output, OutputProps };
