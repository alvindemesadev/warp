import Nav from "@/components/Nav";
import Hero from "@/components/Hero";
import Problem from "@/components/Problem";
import Features from "@/components/Features";
import HowItWorks from "@/components/HowItWorks";
import Download from "@/components/Download";
import Footer from "@/components/Footer";

export default function Home() {
  return (
    <main className="relative z-10">
      <Nav />
      <Hero />
      <Problem />
      <Features />
      <HowItWorks />
      <Download />
      <Footer />
    </main>
  );
}
