import { pageMetadata } from "@/lib/page-metadata";

export const metadata = pageMetadata("remote-view");

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
