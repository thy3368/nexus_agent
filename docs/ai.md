# 屠龙技：我们如何在短短一周内利用人工智能重建 Next.js

**作者**：史蒂夫·福克纳  
**发布日期**：2026-02-24  
**阅读时间**：10分钟

> *本文已于太平洋时间下午 12:35 更新，以修正构建时间基准测试中的拼写错误。*

上周，一位工程师和一个人工智能模型从零开始重构了最流行的前端框架。最终成果 **vinext**（发音为"vee-next"）是基于 Vite 构建的 Next.js 的即插即用型替代方案，只需一条命令即可部署到 Cloudflare Workers。在早期基准测试中，vinext 构建生产应用的速度提升了 4 倍，客户端软件包体积缩小了 57%。目前，我们已经有客户在生产环境中运行 vinext。

整个过程花费了大约 **1100 美元**的代币。

---

## Next.js 部署问题

Next.js 是最流行的 React 框架。数百万开发者都在使用它。它为大量的生产环境 Web 应用提供支持，这并非偶然。它的开发者体验堪称一流。

但 Next.js 在更广泛的无服务器生态系统中部署时存在问题。其工具完全是定制的：Next.js 在 Turbopack 上投入巨资，但如果您想将其部署到 Cloudflare、Netlify 或 AWS Lambda，则必须将构建输出重新格式化为目标平台可以实际运行的格式。

如果你在想："OpenNext 不就是这么做的吗？"，那么你的想法是对的。

这正是 OpenNext 旨在解决的问题。包括我们 Cloudflare 在内的多家供应商都为 OpenNext 投入了大量工程精力。它确实有效，但很快就会遇到各种限制，最终变成一场疲于奔命的"打地鼠"游戏。

以 Next.js 的输出为基础进行开发已被证明是一种困难且脆弱的方法。由于 OpenNext 必须对 Next.js 的构建输出进行逆向工程，这会导致版本之间出现不可预测的差异，而这些差异需要大量工作才能修复。

Next.js 一直在开发一流的适配器 API，我们也一直在与他们合作。虽然目前还处于早期阶段，但即使有了适配器，你仍然需要使用定制的 Turbopack 工具链进行构建。而且适配器仅涵盖构建和部署。在开发过程中，Next.js 开发环境完全运行在 Node.js 中，无法接入其他运行时环境。如果你的应用程序使用了平台特定的 API，例如 Durable Objects、KV 或 AI 绑定，那么在开发环境中，你无法在没有变通方法的情况下测试这些代码。

---

## 介绍 vinext

如果我们不采用 Next.js 的输出格式，而是直接在 Vite 上重新实现 Next.js 的 API 接口呢？Vite 是 Next.js 之外大多数前端生态系统使用的构建工具，为 Astro、SvelteKit、Nuxt 和 Remix 等框架提供支持。我们需要的是完全重新实现，而不仅仅是封装或适配器。说实话，我们一开始并不认为这行得通。但现在是 2026 年，软件开发的成本已经发生了翻天覆地的变化。

我们取得的成就远超预期。

```bash
npm install vinext
```

在你的脚本中替换 `next` 为 `vinext`，其他所有内容保持不变。你现有的 `app/`、`pages/` 和 `next.config.js` 可以照常运行。

```bash
vinext dev          # Development server with HMR
vinext build        # Production build
vinext deploy       # Build and deploy to Cloudflare Workers
```

这并非对 Next.js 和 Turbopack 输出的简单封装，而是 API 接口的另一种实现方式：路由、服务器端渲染、React 服务器组件、服务器端操作、缓存、中间件等等。所有这些都基于 Vite 构建，并以插件的形式提供。最重要的是，得益于 Vite 环境 API，Vite 输出可以在任何平台上运行。

---

## 这些数字

早期基准测试结果令人鼓舞。我们使用一个包含 33 条路由的共享 App Router 应用，将 vinext 与 Next.js 16 进行了对比。两个框架执行相同的工作：编译、打包和准备服务器端渲染的路由。我们在 Next.js 的构建过程中禁用了 TypeScript 类型检查和 ESLint（Vite 在构建过程中不会运行这些），并使用了 `force-dynamic`，以避免 Next.js 花费额外的时间预渲染静态路由，从而避免不公平地降低其性能。我们的目标仅是测量打包和编译速度，不涉及其他任何方面。基准测试在 GitHub CI 上运行，每次合并到主分支时都会执行。

### 生产环境构建时间

| 框架 | 时间 | 与 Next.js 对比 |
|------|------|-----------------|
| Next.js 16.1.6 (Turbopack) | 7.38秒 | 基线 |
| vinext（Vite 7 / Rollup） | 4.64秒 | 速度提升 1.6 倍 |
| vinext（Vite 8 / Rolldown） | 1.67秒 | 速度提升 4.4 倍 |

### 客户端软件包大小（gzip 压缩后）

| 框架 | 大小 | 与 Next.js 对比 |
|------|------|-----------------|
| Next.js 16.1.6 | 168.9 KB | 基线 |
| vinext（Rollup） | 74.0 KB | 缩小了 56% |
| vinext（Rolldown） | 72.9 KB | 缩小了 57% |

这些基准测试衡量的是编译和打包速度，而非生产环境的服务器性能。测试环境是一个包含 33 个路由的单一应用程序，并非所有生产应用程序的代表性样本。随着三个项目的持续开发，我们预计这些数据会不断变化。完整的测试方法和历史结果均已公开。请将其视为参考，而非最终结果。

不过，这个方向令人鼓舞。Vite 的架构，尤其是 Rolldown（Vite 8 中推出的基于 Rust 的打包工具），在构建性能方面具有结构性优势，这一点在这里体现得非常明显。

---

## 部署到 Cloudflare Workers

vinext 的首要部署目标就是 Cloudflare Workers。只需一条命令，即可将源代码部署到正在运行的 Worker 服务器：

```bash
vinext deploy
```

它负责所有操作：构建应用程序、自动生成 Worker 配置并进行部署。App Router 和 Pages Router 都运行在 Worker 上，支持完整的客户端数据填充、交互式组件、客户端导航和 React 状态管理。

对于生产环境缓存，vinext 包含一个 Cloudflare KV 缓存处理程序，可为您提供开箱即用的 ISR（增量静态再生）功能：

```typescript
import { KVCacheHandler } from "vinext/cloudflare";
import { setCacheHandler } from "next/cache";

setCacheHandler(new KVCacheHandler(env.MY_KV_NAMESPACE));
```

对于大多数应用程序来说，KV 是一个不错的默认选择，但缓存层的设计是可插拔的。通过 `setCacheHandler` 调用，您可以根据需要切换任何后端。对于缓存数据量较大或访问模式不同的应用程序，R2 可能更合适。我们也在改进缓存 API，以提供配置更少、功能更强大的缓存层。我们的目标是提供灵活性：您可以根据应用程序的需求选择合适的缓存策略。

### 当前正在运行的实时示例

- [App Router Playground](https://github.com/cloudflare/vinext/tree/main/examples/app-router-playground)
- [Hacker News 克隆版](https://github.com/cloudflare/vinext/tree/main/examples/hacker-news)
- [App Router 最小化](https://github.com/cloudflare/vinext/tree/main/examples/app-router-minimal)
- [Pages Router 最小化](https://github.com/cloudflare/vinext/tree/main/examples/pages-router-minimal)

我们还提供了一个在 Next.js 应用中运行 Cloudflare Agent 的实际示例，无需像 `getPlatformProxy` 这样的变通方法，因为整个应用现在在开发和部署阶段都以 workerd 模式运行。这意味着可以毫无顾虑地使用 Durable Objects、AI 绑定以及所有其他 Cloudflare 特有的服务。[点击此处查看](https://github.com/cloudflare/vinext/tree/main/examples/nextjs-cloudflare-agent)。

---

## 框架设计是一项团队运动

目前的部署目标是 Cloudflare Workers，但这只是冰山一角。Vinext 大约 95% 的功能都基于 Vite。路由、模块适配、SSR 流水线、RSC 集成：这些都不是 Cloudflare 特有的。

Cloudflare 正在寻求与其他托管服务提供商合作，为其客户部署这套工具链（部署难度极低——我们在 Vercel 上仅用了不到 30 分钟就完成了概念验证！）。这是一个开源项目，为了其长期成功，我们认为与整个生态系统的合作伙伴携手合作，确保持续投入至关重要。我们欢迎其他平台提交 PR。如果您有兴趣添加部署目标，请提交 issue 或联系我们。

---

## 状态：实验性

我们想明确指出：vinext 仍处于实验阶段。它推出还不到一周，尚未经过大规模流量的实际测试。如果您正在评估其在生产环境中的应用，请务必谨慎行事。

尽管如此，该测试套件非常全面：包含超过 1700 个 Vitest 测试和 380 个 Playwright 端到端测试，其中包括直接从 Next.js 测试套件和 OpenNext 的 Cloudflare 一致性测试套件移植的测试。我们已使用 Next.js App Router Playground 对其进行了验证。覆盖率达到了 Next.js 16 API 接口的 94%。来自实际客户的早期结果令人鼓舞。我们一直在与 National Design Studio 合作，该团队致力于实现政府所有界面的现代化，我们目前在其测试网站 CIO.gov 上进行测试。他们已经在生产环境中运行 vinext，并在构建时间和包大小方面取得了显著改进。

README 文件坦诚地说明了哪些功能不支持、哪些功能将来也不会支持，以及已知的局限性。我们希望坦诚相待，而不是过度承诺。

---

## 预渲染呢？

vinext 已经开箱即用地支持增量静态重生成 (ISR)。在首次请求任何页面后，页面都会被缓存并在后台重新验证，就像 Next.js 一样。目前这部分功能已经可以正常工作。

vinext 目前尚不支持构建时的静态预渲染。在 Next.js 中，不包含动态数据的页面会在构建过程中渲染 `next build` 并以静态 HTML 的形式提供。如果存在动态路由，则需要 `generateStaticParams()` 预先枚举要构建的页面。vinext 目前还不支持此功能。

这是为了产品发布而特意做出的设计决策。虽然它 已列入开发计划，但如果您的网站完全由预构建的 HTML 静态内容构成，那么目前您可能无法从 Vinext 中获得太多好处。话虽如此，如果一位工程师可以花费 1100 美元购买代币并重新构建 Next.js，那么您或许也只需花费 10 美元即可迁移到专为静态内容设计的基于 Vite 的框架，例如 Astro（它也支持部署到 Cloudflare Workers）。

但对于并非完全静态的网站，我们认为我们可以做得比在构建时预渲染所有内容更好。

---

## 引入交通感知预渲染

Next.js 会在构建过程中预渲染所有列出的页面 `generateStaticParams()`。一个拥有 10,000 个产品页面的网站意味着在构建时会渲染 10,000 次，即使其中 99% 的页面可能永远不会被请求。构建时间与页面数量呈线性增长。这就是为什么大型 Next.js 网站最终需要 30 分钟才能完成构建的原因。

因此，我们开发了交通感知预渲染（TPR）功能。目前它仍处于实验阶段，我们计划在进行更多实际测试后将其设为默认功能。

思路很简单。Cloudflare 已经是您网站的反向代理。我们掌握了您的流量数据，知道哪些页面实际被访问。因此，vinext 不会预渲染所有内容，也不会完全不预渲染，而是在部署时查询 Cloudflare 的区域分析数据，只预渲染那些真正重要的页面。

```bash
vinext deploy --experimental-tpr
```

输出示例：

```
Building...
Build complete (4.2s)

TPR (experimental): Analyzing traffic for my-store.com (last 24h)
TPR: 12,847 unique paths — 184 pages cover 90% of traffic
TPR: Pre-rendering 184 pages...
TPR: Pre-rendered 184 pages in 8.3s → KV cache

Deploying to Cloudflare Workers...
```

对于一个拥有 10 万个产品页面的网站，根据幂律分布，通常 90% 的流量会流向 50 到 200 个页面。这些页面会在几秒钟内完成预渲染。其余页面则采用按需服务端渲染 (SSR)，并在首次请求后通过中断服务端渲染 (ISR) 进行缓存。每次部署都会根据当前的流量模式刷新缓存集。热门页面会被自动抓取。所有这些功能都无需 `generateStaticParams()` 将构建与生产数据库耦合即可运行。

---

## 迎接 Next.js 的挑战，但这次是与人工智能相结合

像这样的项目通常需要一支工程师团队花费数月甚至数年的时间才能完成。许多公司的团队都尝试过，但其规模实在太大了。我们在 Cloudflare 也尝试过一次！它涉及到两台路由器、33 个以上的模块、服务器渲染管线、RSC 流媒体、文件系统路由、中间件、缓存和静态导出。这就是为什么至今无人成功的原因。

这次我们不到一周就完成了。一位工程师（严格来说是工程经理）负责指导人工智能。

第一次提交于 2 月 13 日。当天晚上结束时，Pages Router 和 App Router 的基本 SSR 功能已经实现，中间件、服务器操作和流媒体功能也都已就绪。第二天下午，App Router Playground 已经渲染了 11 个路由中的 10 个。到了第三天，`vinext deploy` 我们已经能够将应用部署到 Cloudflare Workers，并实现了完整的客户端水合。接下来的一周主要进行安全加固：修复各种极端情况，扩展测试套件，并将 API 覆盖率提升至 94%。

与之前的尝试相比，有哪些变化？人工智能变得更好了。好得多。

---

## 为什么说这个问题适合用人工智能来解决

并非每个项目都会如此顺利。这个项目之所以如此，是因为几个因素恰好在合适的时间汇合在一起。

### 1. Next.js 规范完善

它拥有丰富的文档、庞大的用户群，以及多年来在 Stack Overflow 上积累的大量解答和教程。其 API 接口涵盖了所有相关知识。当你让 Claude 实现 `getServerSideProps` 或解释 `useRouter` Next.js 的工作原理时，它不会凭空想象。它了解 Next.js 的运作方式。

### 2. Next.js 拥有完善的测试套件

Next.js 代码库包含数千个端到端测试，涵盖了所有功能和边界情况。我们直接从他们的测试套件中移植了测试（您可以在代码中看到出处）。这为我们提供了一个可以进行机械验证的规范。

### 3. Vite 是一个优秀的基础架构

它处理了前端工具的诸多难点：快速的 HMR、原生 ESM、简洁的插件 API 以及生产环境打包。我们无需自行构建打包工具，只需让它能够识别 Next.js 即可。虽然 `@vitejs/plugin-rsc` 目前还处于早期阶段，但它已经为我们提供了 React Server Components 支持，而无需从头开始构建 RSC 实现。

### 4. 这些模型终于赶上了

我们认为这在几个月前是不可能实现的。早期的模型无法在如此庞大的代码库中保持一致性。新模型能够将整个架构置于上下文中，推断模块之间的交互方式，并能频繁地生成正确的代码，从而保持持续改进的势头。有时，我甚至看到它们深入到 Next、Vite 和 React 的内部代码中去查找 bug。这些最先进的模型令人印象深刻，而且它们似乎还在不断进步。

所有这些条件必须同时满足：完善的目标 API 文档、全面的测试套件、可靠的底层构建工具，以及能够有效应对复杂性的模型。缺少其中任何一项，效果都会大打折扣。

---

## 我们实际是如何建造的

vinext 中的几乎每一行代码都是由 AI 编写的。但更重要的是：每一行代码都通过了与人类编写的代码相同的质量把关。该项目拥有 1700 多个 Vitest 测试、380 个 Playwright 端到端测试、通过 tsgo 实现的完整 TypeScript 类型检查以及通过 oxlint 进行代码检查。持续集成会在每次提交 pull request 时运行所有这些测试。建立一套完善的防护机制对于让 AI 在代码库中高效运行至关重要。

整个过程始于一个计划。我花了几个小时和 OpenCode 的 Claude 反复讨论，最终确定了架构：要构建什么，构建顺序如何，以及使用哪些抽象层。这个计划成为了我们的指路明灯。从那以后，工作流程就变得非常简单明了：

1. 定义一个任务（"使用 `usePathname`、`useSearchParams`、`useRouter` 实现 `next/navigation` 的 shim"）
2. 让人工智能编写实现代码和测试用例
3. 运行测试套件
4. 如果测试通过，则合并。否则，将错误输出传递给 AI，让它迭代处理
5. 重复

我们还为代码审查配备了人工智能代理。当提交 PR 时，一个代理会进行审查。当收到审查意见时，另一个代理会进行处理。反馈循环基本实现了自动化。

它并非每次都能完美运行。有些 PR 就是错的。人工智能会自信地实现一些看似正确但实际上与 Next.js 行为不符的东西。我不得不经常进行纠正。架构决策、优先级排序、判断人工智能何时走入死胡同：这些都由我负责。当你为人工智能提供正确的方向、良好的上下文和完善的规则时，它可以非常高效。但最终，仍然需要人类来掌舵。

为了进行浏览器级别的测试，我使用了 agent-browser 来验证实际渲染的输出、客户端导航和水合行为。单元测试会遗漏很多细微的浏览器问题，而 agent-browser 可以捕捉到这些问题。

项目进行期间，我们在 OpenCode 上运行了 800 多次会话。总成本：约 **1100 美元**的 Claude API 代币。

---

## 这对软件意味着什么

为什么我们的技术栈会有这么多层？这个项目迫使我深入思考这个问题，以及人工智能如何影响答案。

软件中的大多数抽象概念都源于人类的需要。我们无法将整个系统完全装进脑子里，所以我们构建了多层架构来管理其复杂性。每一层都让下一层的工作更轻松。这就是为什么最终会出现层层嵌套的框架、包装库以及成千上万行的粘合代码。

人工智能没有同样的局限性。它可以将整个系统置于上下文中，然后直接编写代码。它不需要中间框架来保持组织性，只需要一个规范和一个基础框架即可。

目前还不清楚哪些抽象概念是真正的基础，哪些只是人类认知的一种辅助手段。这条界限在未来几年内将会发生很大变化。但 vinext 就是一个例证。我们采用了 API 契约、构建工具和 AI 模型，然后由 AI 编写了中间的所有代码。无需任何中间框架。我们认为这种模式将在很多软件中重复出现。我们多年来构建的层级并非都能保留下来。

---

## 致谢

感谢 Vite 团队。Vite 是整个项目的基石。`@vitejs/plugin-rsc` 虽然目前还处于早期阶段，但它为我提供了 RSC 支持，而无需从头开始构建，否则这将是我放弃这个项目的必要条件。在我将插件推向之前从未测试过的领域时，Vite 的维护人员反应迅速且乐于助人。

我们还要特别感谢 Next.js 团队。他们多年来致力于构建一个框架，极大地提升了 React 开发的标准。他们完善的 API 文档和全面的测试套件是该项目得以实现的关键因素。如果没有他们树立的标杆，vinext 也不会存在。

---

## 试试看

vinext 包含一个代理技能，可以自动处理迁移。它支持 Claude Code、OpenCode、Cursor、Codex 以及其他数十种 AI 编码工具。安装后，打开你的 Next.js 项目，然后指示 AI 进行迁移即可：

```bash
npx skills add cloudflare/vinext
```

然后使用任何受支持的工具打开您的 Next.js 项目并执行以下操作：

```
migrate this project to vinext
```

该技能负责兼容性检查、依赖项安装、配置生成和开发服务器启动。它了解 vinext 支持哪些功能，并会标记出任何需要人工干预的问题。

或者，如果您更喜欢手工制作：

```bash
npx vinext init    # Migrate an existing Next.js project
npx vinext dev     # Start the dev server
npx vinext deploy  # Ship to Cloudflare Workers
```

源代码位于 [github.com/cloudflare/vinext](https://github.com/cloudflare/vinext)。欢迎提交问题、PR 和反馈意见。

---

## Todo 总结：一周内利用 AI 重建 Next.js 的关键和步骤

### 关键成功因素

| 因素 | 说明 |
|------|------|
| **完善的目标规范** | Next.js 拥有丰富的文档、庞大用户群、Stack Overflow 解答，AI 能准确理解其 API |
| **全面的测试套件** | Next.js 有数千个端到端测试，可直接移植作为验证规范 |
| **优秀的底层工具** | Vite 提供 HMR、ESM、插件 API、生产打包，无需自建构建工具 |
| **AI 模型能力足够** | 新模型能理解整个架构，推断模块交互，生成正确代码 |

### 项目执行步骤

- [ ] **第 1 步：制定架构计划**
  - 与 AI 讨论确定要构建什么、构建顺序、抽象层设计
  - 花费数小时形成清晰的架构文档

- [ ] **第 2 步：基础功能实现（第 1 天）**
  - Pages Router 和 App Router 基本 SSR
  - 中间件、服务器操作、流媒体功能

- [ ] **第 3 步：路由渲染（第 2 天）**
  - App Router Playground 11 个路由中渲染 10 个

- [ ] **第 4 步：部署能力（第 3 天）**
  - 实现 `vinext deploy` 部署到 Cloudflare Workers
  - 完整的客户端水合

- [ ] **第 5 步：安全加固（其余天数）**
  - 修复边界情况和极端情况
  - 扩展测试套件
  - 提升 API 覆盖率至 94%

### AI 驱动的工作流程

```
1. 定义任务 → 2. AI 编写代码和测试 → 3. 运行测试 → 4. 迭代修复 → 重复
```

- **代码审查**：AI 代理自动审查 PR
- **浏览器测试**：使用 agent-browser 验证渲染输出
- **质量把关**：1700+ Vitest 测试、380 个 Playwright E2E 测试、TypeScript 类型检查、代码 lint

### 最终成果

- **构建速度**：提升 4.4 倍（1.67s vs 7.38s）
- **包体积**：缩小 57%（72.9KB vs 168.9KB）
- **成本**：约 1100 美元 Claude API 代币
- **会话数**：800+ 次 OpenCode 会话

### 重要经验

1. AI 需要**正确的方向、良好的上下文和完善的规则**才能高效
2. **人类仍需掌舵**：架构决策、优先级排序、识别死胡同
3. 不是所有项目都适合 AI——需要目标有完善文档、测试套件、可靠底层工具