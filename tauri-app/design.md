# 提取菜谱页面设计参考

## 整体风格

### Xiaohongshu 风格（主）
- 暖白底色（`#f5f5f5` / `#fafafa`），柔和视觉
- 圆角卡片（`rounded-xl` / `rounded-2xl`），极窄边框 + 轻微阴影
- 品牌色：`#ff2442`（小红书红），点缀使用不可过量
- 字体：无衬线字体，标题 `font-bold`，正文 `font-normal`

### 视觉层次

```
页面结构（从上到下）:

[标题行]   Sparkle icon + "提取菜谱"
[副标题]   灰色小字说明
[搜索框]   圆角输入框 + "提取"按钮（无外层Card包裹）
[结果区]   提取结果卡片列表
```

## 核心UI模式

### 1. URL 输入区（当前重点）

参考：Cookly, SnapChef, Link Recipe

- 搜索框使用大号圆角输入框（`rounded-xl`），内部左侧有link图标
- 按钮使用品牌色（xhs-red），`rounded-lg`
- 输入框 focus 时有高亮描边（ring-xhs/30）
- 搜索框下方显示提示文字，辅助用户了解支持的链接类型
- **无外层 Card 包裹**，直出在页面上

### 2. 提取结果卡片

参考：小红书笔记卡片、Cookly Recipe Card、Elegant Recipe App

每张卡片包含：
- 左侧封面图（`w-16 h-16`，`rounded-lg`，object-cover）
- 右侧内容区：
  - 菜名（加粗，单行截断）
  - 烹饪时间（clock 图标） + 来源链接（最小字号灰色）
  - Tag 标签（Badge 组件，`rounded-full`，最多显示3个）
- 整张卡片可点击，hover 时有轻微上浮效果（`hover:-translate-y-0.5`）
- 无封面图时使用渐变占位（`from-xhs/10 to-orange-50`）

### 3. 空白状态

参考：Elegant Recipe App

- 页面居中显示温和的引导文字
- 不加任何图标或插画，保持简洁
- 灰色（`text-gray-300`），弱化视觉存在感

### 4. 警告提示

参考：小红书 toast 样式

- 浅琥珀色背景（`bg-amber-50`）
- 右侧提供"重新提取"文字链接按钮
- 使用 AnimatePresence 做入场/出场动画

## 色彩系统

```
xhs:       #ff2442 (品牌红)
xhs-hover: #d61f38 (红 hover)
background: #fafafa (页面底色)
card-bg:   #ffffff (卡片白)
text-main: #111827 (主要文字)
text-sub:  #9ca3af (灰色辅助文字)
border:    #e5e7eb (卡片边框)
amber-bg:  #fffbeb (警告背景)
amber-border: #fde68a (警告边框)
amber-text: #92400e (警告文字)
```

## 动效参考

- 结果卡片入场：fade + 轻微上移（`y: 12 → 0`），逐卡延迟 `i * 0.08s`
- 结果区块入场：fade + 上移（`y: 16 → 0`），延迟 0.15s
- Hover 卡片：向上位移 `-0.5px` + shadow 加深（0.2s ease）
- 警告提示：fade + 上下滑动（0.2s）

## 间距规则

```
页面左右 padding:   p-6 (AppLayout 层)
标题下间距:         mb-1
副标题下间距:       mb-3
提取结果区块上间距:  mt-8
结果卡片间距:       gap-3
结果卡片内间距:     p-3
```

## 参考链接

- [Cookly — Modern Recipe & Cooking App (Dribbble)](https://dribbble.com/shots/26964111-Cookly-Modern-Recipe-Cooking-Mobile-App-UI-Exploration) — 极简、温暖、低认知负荷的设计风格
- [Elegant Recipe App UI (Dribbble)](https://dribbble.com/shots/27297279-Elegant-Recipe-App-UI-Minimal-Mobile-Experience) — 柔和色调、圆角卡片、干净排版
- [Food Mastery Hub (Dribbble)](https://dribbble.com/shots/27295231-Food-Mastery-Hub-Recipe-Mobile-App-UI-Design) — 分类导航、视觉层次分明
- [Modern Recipe Platform (Dribbble)](https://dribbble.com/shots/25825725-Modern-Recipe-Platform-Website-UI) — Content-first 布局，留白充足
- [SnapChef (99designs)](https://99designs.ie/mobile-app-design/contests/design-snapchef-app-modern-minimalistic-user-friendly-recipe-1285940) — 绿色主色调、input 为中心的简洁设计
- [Link Recipe (Devpost)](https://devpost.com/software/link-recipe) — URL 提取菜谱流程、breathing 加载动画
- [Cooking App Issue #79 (GitHub)](https://github.com/nmohamaya/Cooking_app/issues/79) — 完整的提取流程 UI 规范（paste → preview → edit → save）
