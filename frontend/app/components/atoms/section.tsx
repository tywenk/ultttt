import { type Cell } from "@/types";

export function Cell({
  section,
  children,
}: {
  section: Cell;
  children: React.ReactNode;
}) {
  return <div className="">{children}</div>;
}
