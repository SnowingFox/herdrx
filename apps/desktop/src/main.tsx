import { createRoot } from "react-dom/client";
import { init } from "ghostty-web";
import "./index.css";
import { Spike } from "./terminal/Spike";

await init();
createRoot(document.getElementById("root")!).render(<Spike />);
