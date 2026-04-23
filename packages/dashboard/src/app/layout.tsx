import type { Metadata } from "next";
import "./globals.css";
import { JetBrains_Mono, Manrope } from "next/font/google";
import { cn } from "@/lib/utils";
import { TooltipProvider } from "@/components/ui/tooltip";
import { JotaiProvider } from "@/store/provider";
import { ThemeProvider } from "@/components/theme-provider";

const manrope = Manrope({ subsets: ["latin"], variable: "--font-sans" });
const jetbrainsMono = JetBrains_Mono({ subsets: ["latin"], variable: "--font-mono" });

export const metadata: Metadata = {
  title: "agent-browser",
  description: "Observability dashboard for agent-browser",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html
      lang="en"
      className={cn("font-sans antialiased", manrope.variable, jetbrainsMono.variable)}
      suppressHydrationWarning
    >
      <body>
        <ThemeProvider attribute="class" defaultTheme="dark" enableSystem disableTransitionOnChange>
          <JotaiProvider>
            <TooltipProvider>{children}</TooltipProvider>
          </JotaiProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
