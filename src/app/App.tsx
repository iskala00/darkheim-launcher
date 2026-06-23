import { CircleFill } from "@gravity-ui/icons";
import { LaunchPanel } from "@/components/LaunchPanel";
import Footer from "@/components/layouts/footer/Footer";

import "./styles/global.css";
import "./styles/main.scss";

function App() {
  return (
    <main className="flex flex-col justify-between h-full">
      <div className="z-1">
        <img src="/logo.png" alt="Darkheim Logo" width={200} />
      </div>
      <div className="text-center">
        <div className="flex flex-col items-center gap-4">
          <LaunchPanel />
          <span className="flex self-center items-center gap-2">
            <div className="relative inline-flex size-[15px] items-center justify-center">
              <span className="absolute left-1/2 top-1/2 size-[10px] -translate-x-1/2 -translate-y-1/2 animate-ping rounded-full bg-success opacity-75" />

              <CircleFill
                className="relative text-success"
                width={15}
                height={15}
              />
            </div>
            <span className="text-white text-sm">Серверы онлайн</span>
          </span>
        </div>
        <Footer />
      </div>
    </main>
  );
}

export default App;
