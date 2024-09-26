import { ThemeProvider } from "next-themes";
import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Lox Playground",
  description: "Run Lox code in your browser.",
  metadataBase: new URL("https://ajeetdsouza.github.io/loxcraft/"),
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>
        <ThemeProvider attribute="class" defaultTheme="dark">
          {children}
        </ThemeProvider>
      </body>
    </html>
  );
}
