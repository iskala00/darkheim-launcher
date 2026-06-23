import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CircleFill } from "@gravity-ui/icons";
import { PlayButton } from "@/components/PlayButton";
import Footer from "@/components/layouts/footer/Footer";

import "./styles/global.css";
import "./styles/main.scss";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main className="flex flex-col justify-between h-full">
      <div>
        <img src="/logo.png" alt="Darkheim Logo" width={200} />
      </div>
      <div className="text-center">
        <div className="flex flex-col items-center gap-4">
          <PlayButton />
          <span className="flex items-center gap-2">
            <CircleFill className="text-success" width={16} height={16} />
            <span className="text-white text-sm">Серверы онлайн</span>
          </span>
        </div>
        <Footer />
      </div>
    </main>
  );
}

export default App;
