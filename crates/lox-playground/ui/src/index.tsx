import { App } from "./App";
import { createRoot } from "react-dom/client";
import "./index.css";

const container = document.getElementById("app");
const root = createRoot(container!);
root.render(<App />);
