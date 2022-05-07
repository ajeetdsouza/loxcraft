import React, { useEffect, useState } from "react";
import init, { add } from "lox-wasm";

export function App() {
  const [ans, setAns] = useState(0);
  useEffect(() => {
    init().then(() => {
      setAns(add(1, 1));
    });
  }, []);
  return <h1>{ans}</h1>;
}
