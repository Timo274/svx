import { Hero } from "./components/Hero";
import { Steps } from "./components/Steps";
import { Security } from "./components/Security";
import { HotSwap } from "./components/HotSwap";
import { Footer } from "./components/Footer";
import { RelayStatus } from "./components/RelayStatus";

export default function App() {
  return (
    <div className="min-h-screen">
      <main className="mx-auto w-full max-w-5xl px-6 py-14 md:py-20 space-y-24">
        <Hero />
        <Steps />
        <Security />
        <HotSwap />
        <RelayStatus />
      </main>
      <Footer />
    </div>
  );
}
