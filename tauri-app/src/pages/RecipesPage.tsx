import { useState, useEffect, useCallback, useMemo } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Search, X, Clock, AlertTriangle, BookOpen } from "lucide-react";
import { toast } from "sonner";
import { listRecipes, getRecipe, deleteRecipe, saveRecipe } from "@/lib/tauri";
import { truncateUrl } from "@/lib/helpers";
import { searchRecipes, filterByTag, collectTags } from "@/lib/searchUtils";
import { getFavorites, favKey } from "@/lib/favorites";
import CookingPage from "@/pages/CookingPage";
import type { Recipe, RecipeSummary } from "@/types/recipe";

export default function RecipesPage() {
  const [allRecipes, setAllRecipes] = useState<RecipeSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState("");
  const [activeTag, setActiveTag] = useState<string | null>(null);
  const [favoritesOnly, setFavoritesOnly] = useState(false);
  const [selectedRecipe, setSelectedRecipe] = useState<Recipe | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<RecipeSummary | null>(null);

  const load = useCallback(async () => {
    try {
      setLoading(true);
      setAllRecipes(await listRecipes());
    } catch (err) {
      console.error("Failed to load recipes:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const tags = useMemo(() => collectTags(allRecipes), [allRecipes]);

  const [favorites, setFavorites] = useState<Set<string>>(getFavorites);

  const filtered = useMemo(() => {
    let result = searchRecipes(allRecipes, query);
    result = filterByTag(result, activeTag);
    if (favoritesOnly) {
      result = result.filter((r) => favorites.has(favKey(r.source_url, r.name)));
    }
    return result;
  }, [allRecipes, query, activeTag, favoritesOnly, favorites]);

  const handleView = async (summary: RecipeSummary) => {
    try {
      const recipe = await getRecipe(summary.id);
      setSelectedRecipe(recipe);
    } catch (err) {
      console.error("Failed to load recipe:", err);
    }
  };

  const handleDelete = async (summary: RecipeSummary) => {
    setDeleteConfirm(summary);
  };

  const confirmDelete = async () => {
    if (!deleteConfirm) return;
    try {
      // Fetch full recipe first in case user wants to undo
      const recipe = await getRecipe(deleteConfirm.id);
      await deleteRecipe(deleteConfirm.id);
      setAllRecipes((prev) => prev.filter((r) => r.id !== deleteConfirm.id));
      toast.success(`已删除「${deleteConfirm.name}」`, {
        action: {
          label: "撤销",
          onClick: async () => {
            try {
              await saveRecipe(recipe);
              load();
              toast.success(`已恢复「${deleteConfirm.name}」`);
            } catch {
              toast.error("撤销失败，请重新提取");
            }
          },
        },
        duration: 4000,
      });
    } catch (err) {
      console.error("Failed to delete:", err);
      toast.error(`删除失败`);
    } finally {
      setDeleteConfirm(null);
    }
  };

  // Show recipe detail inline when one is selected
  if (selectedRecipe) {
    return (
      <CookingPage
        recipe={selectedRecipe}
        onBack={() => setSelectedRecipe(null)}
      />
    );
  }

  if (loading) {
    return (
      <div>
        <div className="flex items-center gap-2 mb-1">
          <BookOpen size={22} className="text-xhs" />
          <h2 className="text-[22px] font-bold text-xhs">我的菜谱</h2>
        </div>
        <p className="text-sm text-gray-400 mb-3">查看和管理已保存的菜谱</p>
        <div className="animate-pulse space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="h-20 bg-gray-100 rounded-xl" />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div>
      <div className="flex items-center gap-2 mb-1">
        <BookOpen size={22} className="text-xhs" />
        <h2 className="text-[22px] font-bold text-xhs">我的菜谱</h2>
      </div>
      <p className="text-sm text-gray-400 mb-3">查看和管理已保存的菜谱</p>

      {/* Search bar */}
      <div className="relative mb-3">
        <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="搜索菜谱、食材..."
          className="pl-9 pr-9 h-10 text-sm rounded-xl"
        />
        {query && (
          <button
            onClick={() => setQuery("")}
            className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
          >
            <X size={16} />
          </button>
        )}
      </div>

      {/* Tag filter chips */}
      <div className="flex flex-wrap gap-1.5 mb-4">
        <Badge
          variant={!favoritesOnly && activeTag === null ? "default" : "secondary"}
          className="cursor-pointer"
          onClick={() => { setFavoritesOnly(false); setActiveTag(null); }}
        >
          全部
        </Badge>
        <Badge
          variant={favoritesOnly ? "default" : "secondary"}
          className="cursor-pointer"
          onClick={() => setFavoritesOnly(!favoritesOnly)}
        >
          ⭐ 收藏
        </Badge>
        {tags.map((tag) => (
          <Badge
            key={tag}
            variant={!favoritesOnly && activeTag === tag ? "default" : "secondary"}
            className="cursor-pointer"
            onClick={() => { setFavoritesOnly(false); setActiveTag(activeTag === tag ? null : tag); }}
          >
            {tag}
          </Badge>
        ))}
      </div>

      {/* Results */}
      {allRecipes.length === 0 ? (
        <div className="text-center py-16 text-gray-300">
          <p className="text-sm font-medium">还没有保存的菜谱</p>
          <p className="text-xs mt-2">去「提取菜谱」提取第一条菜谱吧</p>
        </div>
      ) : filtered.length === 0 ? (
        <div className="text-center py-16 text-gray-300">
          <p className="text-sm font-medium">
            {favoritesOnly ? "还没有收藏的菜谱" : "没有找到相关菜谱"}
          </p>
          <p className="text-xs mt-2">
            {favoritesOnly
              ? "在菜谱详情页点击⭐即可收藏"
              : `试试搜索「${allRecipes[0]?.name?.slice(0, 2) || "排骨"}」`}
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-3">
          {filtered.map((r) => (
            <Card
              key={r.id}
              className="cursor-pointer hover:shadow-md transition-all duration-200 hover:-translate-y-0.5 overflow-hidden"
              onClick={() => handleView(r)}
            >
              {/* Cover image */}
              <div className="h-28 bg-gradient-to-br from-xhs/10 to-orange-50 relative overflow-hidden">
                {r.cover_image_url && (
                  <img
                    src={r.cover_image_url}
                    alt={r.name}
                    className="w-full h-full object-cover"
                    onError={(e) => {
                      (e.target as HTMLImageElement).style.display = "none";
                    }}
                  />
                )}
                {r.total_time && (
                  <span className="absolute bottom-1.5 right-1.5 bg-black/50 backdrop-blur-sm text-white text-[10px] px-1.5 py-0.5 rounded-full flex items-center gap-0.5">
                    <Clock size={10} />
                    {r.total_time}
                  </span>
                )}
              </div>
              <CardContent className="p-2.5 space-y-1">
                <div className="flex items-start justify-between gap-1">
                  <p className="font-semibold text-sm leading-tight line-clamp-2">
                    {r.name}
                  </p>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDelete(r);
                    }}
                    className="text-gray-300 hover:text-red-500 shrink-0 transition-colors"
                    title="删除"
                  >
                    <X size={14} />
                  </button>
                </div>
                <p className="text-[10px] text-gray-400 truncate">
                  {truncateUrl(r.source_url, 30)}
                </p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Delete confirmation modal */}
      <AnimatePresence>
        {deleteConfirm && (
          <motion.div
            key="delete-modal"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
            className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4"
            onClick={() => setDeleteConfirm(null)}
          >
            <motion.div
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: 12 }}
              transition={{ duration: 0.2, ease: "easeOut" }}
              className="bg-white rounded-2xl p-5 max-w-sm w-full shadow-xl"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="flex items-start gap-3">
                <div className="w-10 h-10 rounded-full bg-xhs/10 flex items-center justify-center shrink-0">
                  <AlertTriangle size={20} className="text-xhs" />
                </div>
                <div className="min-w-0 flex-1 pt-0.5">
                  <h3 className="font-semibold text-[15px] text-gray-900">确认删除</h3>
                  <p className="text-sm text-gray-500 mt-1 leading-relaxed">
                    确定删除「{deleteConfirm.name}」？此操作不可撤回。
                  </p>
                </div>
              </div>
              <div className="flex gap-2 mt-5 justify-end">
                <button
                  onClick={() => setDeleteConfirm(null)}
                  className="px-4 py-2 text-sm font-medium rounded-xl bg-gray-100 hover:bg-gray-200 text-gray-700 transition-colors"
                >
                  取消
                </button>
                <button
                  onClick={confirmDelete}
                  className="px-4 py-2 text-sm font-medium rounded-xl bg-xhs hover:bg-xhs-hover text-white transition-colors"
                >
                  删除
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
