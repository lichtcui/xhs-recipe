import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";

interface NavBarProps {
  currentPage: string;
  onNavigate: (page: "home" | "settings") => void;
}

export default function NavBar({ currentPage, onNavigate }: NavBarProps) {
  return (
    <nav className="bg-white border-b px-4">
      <Tabs
        value={currentPage}
        onValueChange={(v) => {
          if (v === "home" || v === "settings") onNavigate(v);
        }}
      >
        <TabsList className="h-auto gap-0 rounded-none bg-transparent p-0">
          <TabsTrigger
            value="home"
            className="rounded-none border-b-2 border-transparent px-5 py-3 text-[15px] data-[state=active]:border-xhs data-[state=active]:text-xhs data-[state=active]:shadow-none"
          >
            首页
          </TabsTrigger>
          <TabsTrigger
            value="settings"
            className="rounded-none border-b-2 border-transparent px-5 py-3 text-[15px] data-[state=active]:border-xhs data-[state=active]:text-xhs data-[state=active]:shadow-none"
          >
            设置
          </TabsTrigger>
        </TabsList>
      </Tabs>
    </nav>
  );
}
