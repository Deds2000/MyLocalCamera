import Shell from "@/app/Shell";
import { useTauriEvents } from "@/hooks/useTauriEvents";

function App() {
  useTauriEvents();
  return <Shell />;
}

export default App;
