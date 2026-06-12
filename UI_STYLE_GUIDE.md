# GeekZip — 荧光极客风格开发指南

> 基于设计图提取的暗黑科技风 UI 实现方案  
> 技术栈: React + Tailwind CSS + CSS Animations

---

## 1. 色彩体系

### 1.1 主色板

```css
:root {
  /* 背景层 — 从深到浅 */
  --bg-primary: #0A0A0F;      /* 最底层背景 */
  --bg-secondary: #111118;    /* 卡片/面板背景 */
  --bg-tertiary: #1A1A24;     /* 悬浮层/输入框背景 */
  --bg-hover: #252532;        /* hover 状态 */
  --bg-active: #2D2D3A;       /* active/选中状态 */

  /* 主色 — 荧光绿 */
  --neon-green: #00E676;
  --neon-green-dim: #00C853;
  --neon-green-bright: #69F0AE;
  --neon-green-dark: #009624;
  --neon-green-glow: rgba(0, 230, 118, 0.4);
  --neon-green-glow-soft: rgba(0, 230, 118, 0.15);

  /* 辅助色 */
  --neon-blue: #2962FF;
  --neon-blue-glow: rgba(41, 98, 255, 0.4);
  --neon-orange: #FF6D00;
  --neon-orange-glow: rgba(255, 109, 0, 0.4);
  --neon-red: #D50000;
  --neon-red-glow: rgba(213, 0, 0, 0.4);
  --neon-purple: #AA00FF;
  --neon-purple-glow: rgba(170, 0, 255, 0.4);

  /* 文字 */
  --text-primary: #FFFFFF;
  --text-secondary: #B0BEC5;
  --text-muted: #607D8B;
  --text-disabled: #37474F;

  /* 边框 */
  --border-subtle: #2D2D3A;
  --border-default: #3D3D4F;
  --border-focus: var(--neon-green);
}
```

### 1.2 Tailwind 配置

```js
// tailwind.config.js
module.exports = {
  content: ['./src/**/*.{js,jsx,ts,tsx}'],
  theme: {
    extend: {
      colors: {
        bg: {
          primary: '#0A0A0F',
          secondary: '#111118',
          tertiary: '#1A1A24',
          hover: '#252532',
          active: '#2D2D3A',
        },
        neon: {
          green: '#00E676',
          'green-dim': '#00C853',
          'green-bright': '#69F0AE',
          'green-dark': '#009624',
          blue: '#2962FF',
          orange: '#FF6D00',
          red: '#D50000',
          purple: '#AA00FF',
        },
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'SF Mono', 'Consolas', 'monospace'],
        display: ['SF Pro Display', 'Segoe UI', 'system-ui', 'sans-serif'],
        pixel: ['VT323', 'Press Start 2P', 'monospace'],
      },
      boxShadow: {
        'neon-green': '0 0 10px rgba(0, 230, 118, 0.4), 0 0 20px rgba(0, 230, 118, 0.2)',
        'neon-green-lg': '0 0 20px rgba(0, 230, 118, 0.5), 0 0 40px rgba(0, 230, 118, 0.3)',
        'neon-green-sm': '0 0 5px rgba(0, 230, 118, 0.3)',
        'neon-blue': '0 0 10px rgba(41, 98, 255, 0.4)',
        'neon-red': '0 0 10px rgba(213, 0, 0, 0.4)',
        'neon-orange': '0 0 10px rgba(255, 109, 0, 0.4)',
        'inner-glow': 'inset 0 0 20px rgba(0, 230, 118, 0.05)',
      },
      textShadow: {
        'neon-green': '0 0 10px rgba(0, 230, 118, 0.8), 0 0 20px rgba(0, 230, 118, 0.4)',
        'neon-green-sm': '0 0 5px rgba(0, 230, 118, 0.6)',
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'scan-line': 'scanLine 8s linear infinite',
        'glow-pulse': 'glowPulse 2s ease-in-out infinite',
        'progress-shine': 'progressShine 2s linear infinite',
        'blink': 'blink 1s step-end infinite',
        'float': 'float 3s ease-in-out infinite',
        'border-flow': 'borderFlow 3s linear infinite',
      },
      keyframes: {
        glowPulse: {
          '0%, 100%': { boxShadow: '0 0 10px rgba(0, 230, 118, 0.4)' },
          '50%': { boxShadow: '0 0 25px rgba(0, 230, 118, 0.6), 0 0 50px rgba(0, 230, 118, 0.2)' },
        },
        progressShine: {
          '0%': { backgroundPosition: '-200% 0' },
          '100%': { backgroundPosition: '200% 0' },
        },
        scanLine: {
          '0%': { transform: 'translateY(-100%)' },
          '100%': { transform: 'translateY(100%)' },
        },
        blink: {
          '0%, 100%': { opacity: '1' },
          '50%': { opacity: '0' },
        },
        float: {
          '0%, 100%': { transform: 'translateY(0px)' },
          '50%': { transform: 'translateY(-10px)' },
        },
        borderFlow: {
          '0%': { backgroundPosition: '0% 50%' },
          '100%': { backgroundPosition: '200% 50%' },
        },
      },
    },
  },
  plugins: [
    require('tailwindcss-textshadow'),
  ],
};
```

---

## 2. 核心视觉效果

### 2.1 霓虹发光 (Neon Glow)

```css
/* 基础霓虹发光 */
.neon-glow {
  box-shadow: 
    0 0 5px var(--neon-green),
    0 0 10px var(--neon-green-glow),
    0 0 20px var(--neon-green-glow-soft);
}

/* 文字霓虹发光 */
.neon-text {
  color: var(--neon-green);
  text-shadow: 
    0 0 5px var(--neon-green),
    0 0 10px var(--neon-green-glow),
    0 0 20px var(--neon-green-glow-soft);
}

/* 边框霓虹发光 */
.neon-border {
  border: 1px solid var(--neon-green);
  box-shadow: 
    inset 0 0 10px var(--neon-green-glow-soft),
    0 0 10px var(--neon-green-glow);
}

/* 脉冲发光 */
.neon-pulse {
  animation: glowPulse 2s ease-in-out infinite;
}
```

### 2.2 点阵网格背景 (Dot Grid)

```css
/* 主区域点阵背景 */
.dot-grid-bg {
  background-color: var(--bg-secondary);
  background-image: 
    radial-gradient(circle, rgba(0, 230, 118, 0.15) 1px, transparent 1px);
  background-size: 20px 20px;
}

/* 更细密的点阵 */
.dot-grid-bg-dense {
  background-color: var(--bg-secondary);
  background-image: 
    radial-gradient(circle, rgba(0, 230, 118, 0.1) 0.5px, transparent 0.5px);
  background-size: 12px 12px;
}

/* 扫描线叠加 */
.scan-line-overlay {
  position: relative;
  overflow: hidden;
}

.scan-line-overlay::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: linear-gradient(
    to bottom,
    transparent,
    rgba(0, 230, 118, 0.1),
    transparent
  );
  animation: scanLine 8s linear infinite;
  pointer-events: none;
}
```

### 2.3 像素字体效果

```css
/* 安装 Google Fonts: VT323, Press Start 2P */
@import url('https://fonts.googleapis.com/css2?family=VT323&family=Press+Start+2P&display=swap');

.pixel-text {
  font-family: 'VT323', 'Press Start 2P', monospace;
  font-size: 2rem;
  letter-spacing: 0.1em;
  color: var(--neon-green);
  text-shadow: 
    0 0 10px var(--neon-green),
    0 0 20px var(--neon-green-glow);
}

/* 复古像素风 */
.retro-pixel {
  font-family: 'Press Start 2P', monospace;
  font-size: 0.75rem;
  line-height: 1.6;
  color: var(--neon-green);
  text-shadow: 2px 2px 0px rgba(0, 0, 0, 0.5);
}
```

### 2.4 进度条发光

```css
/* 荧光进度条 */
.progress-bar-neon {
  height: 8px;
  background: var(--bg-tertiary);
  border-radius: 4px;
  overflow: hidden;
  position: relative;
}

.progress-bar-neon .fill {
  height: 100%;
  background: linear-gradient(90deg, var(--neon-green-dim), var(--neon-green));
  border-radius: 4px;
  box-shadow: 
    0 0 10px var(--neon-green-glow),
    0 0 20px var(--neon-green-glow-soft);
  transition: width 0.3s ease;
  position: relative;
}

/* 进度条上的光条扫过效果 */
.progress-bar-neon .fill::after {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(
    90deg,
    transparent,
    rgba(255, 255, 255, 0.3),
    transparent
  );
  background-size: 200% 100%;
  animation: progressShine 2s linear infinite;
}
```

---

## 3. React 组件实现

### 3.1 主布局 (Dashboard)

```tsx
// components/Dashboard.tsx
import React from 'react';
import Sidebar from './Sidebar';
import MainArea from './MainArea';
import RightPanel from './RightPanel';
import StatusBar from './StatusBar';

export default function Dashboard() {
  return (
    <div className="h-screen w-screen flex flex-col bg-bg-primary text-text-primary font-display overflow-hidden">
      {/* 顶部标题栏 */}
      <header className="h-12 flex items-center px-4 border-b border-border-subtle bg-bg-secondary">
        <div className="flex items-center gap-3">
          <div className="w-3 h-3 rounded-full bg-neon-red animate-pulse" />
          <div className="w-3 h-3 rounded-full bg-neon-orange animate-pulse delay-75" />
          <div className="w-3 h-3 rounded-full bg-neon-green animate-pulse delay-150" />
          <span className="font-mono text-lg font-bold tracking-wider text-white ml-2">
            GeekZip
          </span>
          <span className="px-2 py-0.5 text-xs font-mono border border-neon-green text-neon-green rounded bg-neon-green/10">
            PRO
          </span>
        </div>
        
        <div className="flex-1 flex justify-center">
          <div className="flex bg-bg-tertiary rounded-lg p-0.5 border border-border-subtle">
            {['NORMAL', 'PRO', 'TERMINAL'].map((tab) => (
              <button
                key={tab}
                className={`px-4 py-1.5 text-xs font-mono rounded-md transition-all ${
                  tab === 'PRO'
                    ? 'bg-neon-green/20 text-neon-green border border-neon-green/50'
                    : 'text-text-muted hover:text-text-secondary'
                }`}
              >
                {tab}
              </button>
            ))}
          </div>
        </div>
        
        <div className="flex items-center gap-3">
          <WaveformIcon className="w-5 h-5 text-neon-green" />
          <SettingsIcon className="w-5 h-5 text-text-muted hover:text-text-primary" />
          <div className="w-8 h-8 rounded-full bg-bg-tertiary border border-border-default flex items-center justify-center">
            <UserIcon className="w-4 h-4 text-text-muted" />
          </div>
        </div>
      </header>

      {/* 主内容区 */}
      <div className="flex-1 flex overflow-hidden">
        <Sidebar />
        <MainArea />
        <RightPanel />
      </div>

      {/* 底部状态栏 */}
      <StatusBar />
    </div>
  );
}
```

### 3.2 拖拽区域 (Drop Zone)

```tsx
// components/DropZone.tsx
import React, { useState } from 'react';

export default function DropZone() {
  const [isDragging, setIsDragging] = useState(false);

  return (
    <div
      className={`
        flex-1 flex flex-col items-center justify-center
        dot-grid-bg-dense
        border-2 border-dashed rounded-lg
        transition-all duration-300
        ${isDragging 
          ? 'border-neon-green bg-neon-green/5 shadow-neon-green-lg' 
          : 'border-border-default bg-bg-secondary/50'
        }
      `}
      onDragEnter={() => setIsDragging(true)}
      onDragLeave={() => setIsDragging(false)}
      onDragOver={(e) => e.preventDefault()}
      onDrop={(e) => {
        e.preventDefault();
        setIsDragging(false);
        // 处理文件
      }}
    >
      {/* 归档图标 */}
      <div className="mb-6 relative">
        <div className="w-20 h-24 border-2 border-neon-green rounded-lg flex flex-col items-center justify-center relative
          shadow-neon-green animate-glow-pulse">
          <div className="absolute top-0 left-0 right-0 h-6 border-b-2 border-neon-green/50 bg-neon-green/10 rounded-t-lg" />
          <ArchiveIcon className="w-8 h-8 text-neon-green mt-4" />
        </div>
        {/* 悬浮动画 */}
        <div className="absolute -bottom-2 left-1/2 -translate-x-1/2 w-16 h-1 bg-neon-green/30 rounded-full blur-sm" />
      </div>

      {/* 像素风格标题 */}
      <h2 className="pixel-text text-3xl mb-4 text-center">
        DROP ARCHIVES HERE
      </h2>
      
      <p className="font-mono text-sm text-text-muted mb-8 tracking-widest uppercase">
        OR CLICK TO BROWSE FILES
      </p>

      {/* 按钮组 */}
      <div className="flex gap-4">
        <button className="px-6 py-3 bg-bg-tertiary border border-border-default rounded-md
          font-mono text-sm text-text-secondary hover:border-neon-green hover:text-neon-green
          hover:shadow-neon-green transition-all duration-300 flex items-center gap-2">
          <FileIcon className="w-4 h-4" />
          IMPORT FILES
          <span className="px-1.5 py-0.5 bg-bg-active rounded text-xs text-text-muted">⌘ O</span>
        </button>
        <button className="px-6 py-3 bg-bg-tertiary border border-border-default rounded-md
          font-mono text-sm text-text-secondary hover:border-neon-green hover:text-neon-green
          hover:shadow-neon-green transition-all duration-300 flex items-center gap-2">
          <FolderIcon className="w-4 h-4" />
          IMPORT FOLDER
          <span className="px-1.5 py-0.5 bg-bg-active rounded text-xs text-text-muted">⌘ ⇧ O</span>
        </button>
      </div>
    </div>
  );
}
```

### 3.3 任务卡片 (Task Card)

```tsx
// components/TaskCard.tsx
import React from 'react';

interface TaskCardProps {
  name: string;
  size: string;
  files: number;
  format: string;
  encryption?: string;
  progress: number;
  speed: string;
  eta: string;
  status: 'analyzing' | 'extracting' | 'verifying' | 'error';
  formatColor: string;
}

export default function TaskCard({
  name, size, files, format, encryption, progress, speed, eta, status, formatColor
}: TaskCardProps) {
  const statusText = {
    analyzing: 'Analyzing...',
    extracting: 'Extracting...',
    verifying: 'Verifying...',
    error: 'Error',
  };

  return (
    <div className="p-4 bg-bg-secondary border border-border-subtle rounded-lg 
      hover:border-border-default transition-all group">
      <div className="flex items-start gap-4">
        {/* 格式图标 */}
        <div className={`w-12 h-12 rounded-lg flex items-center justify-center
          border border-${formatColor} bg-${formatColor}/10 shadow-neon-${formatColor}`}>
          <span className={`font-mono text-sm font-bold text-${formatColor}`}>
            {format}
          </span>
        </div>

        {/* 信息 */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between mb-1">
            <h3 className="font-mono text-sm text-neon-green truncate">
              {name}
            </h3>
            <div className="flex items-center gap-2">
              <span className="font-mono text-xs text-text-muted">
                {statusText[status]}
              </span>
              <span className="font-mono text-xs text-neon-green">
                {speed}
              </span>
            </div>
          </div>
          
          <div className="flex items-center gap-2 mb-3 font-mono text-xs text-text-muted">
            <span>{size}</span>
            <span>•</span>
            <span>{files} files</span>
            <span>•</span>
            <span className="text-neon-blue">{format.toUpperCase()}</span>
            {encryption && (
              <>
                <span>•</span>
                <span className="text-neon-orange">{encryption}</span>
              </>
            )}
          </div>

          {/* 进度条 */}
          <div className="flex items-center gap-3">
            <div className="flex-1 h-2 bg-bg-tertiary rounded-full overflow-hidden">
              <div 
                className="h-full bg-gradient-to-r from-neon-green-dim to-neon-green 
                  rounded-full shadow-neon-green transition-all duration-300"
                style={{ width: `${progress}%` }}
              >
                {/* 光条效果 */}
                <div className="h-full w-full animate-progress-shine"
                  style={{
                    background: 'linear-gradient(90deg, transparent, rgba(255,255,255,0.3), transparent)',
                    backgroundSize: '200% 100%',
                  }}
                />
              </div>
            </div>
            <span className="font-mono text-sm text-neon-green w-10 text-right">
              {progress}%
            </span>
          </div>
          
          <div className="flex justify-between mt-2 font-mono text-xs text-text-muted">
            <span>ETA {eta}</span>
            <span className="text-neon-green">{speed}</span>
          </div>
        </div>

        {/* 操作按钮 */}
        <div className="flex items-center gap-2">
          <button className="w-8 h-8 rounded-lg bg-bg-tertiary border border-border-subtle
            flex items-center justify-center hover:border-neon-green hover:text-neon-green
            transition-all">
            <PauseIcon className="w-4 h-4" />
          </button>
          <button className="w-8 h-8 rounded-lg bg-bg-tertiary border border-border-subtle
            flex items-center justify-center hover:border-neon-red hover:text-neon-red
            transition-all">
            <XIcon className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}
```

### 3.4 侧边栏导航 (Sidebar)

```tsx
// components/Sidebar.tsx
import React from 'react';

const archiveItems = [
  { label: 'Inbox', count: 8, icon: InboxIcon },
  { label: 'Processing', count: 2, icon: ProcessingIcon },
  { label: 'Extracted', count: 34, icon: ExtractedIcon },
  { label: 'Favorites', count: 0, icon: StarIcon },
];

const toolItems = [
  { label: 'Compress', icon: CompressIcon },
  { label: 'Extract', icon: ExtractIcon },
  { label: 'Encrypt', icon: EncryptIcon },
  { label: 'Batch', icon: BatchIcon },
];

const aiItems = [
  { label: 'Analyze', icon: AnalyzeIcon },
  { label: 'Organize', icon: OrganizeIcon },
  { label: 'Clean', icon: CleanIcon },
  { label: 'Duplicates', icon: DuplicateIcon },
];

export default function Sidebar() {
  return (
    <div className="w-56 bg-bg-secondary border-r border-border-subtle flex flex-col">
      {/* ARCHIVES 组 */}
      <div className="px-3 py-4">
        <h3 className="font-mono text-xs text-neon-green mb-3 tracking-wider uppercase">
          Archives
        </h3>
        {archiveItems.map((item) => (
          <NavItem key={item.label} {...item} />
        ))}
      </div>

      {/* TOOLS 组 */}
      <div className="px-3 py-4 border-t border-border-subtle">
        <h3 className="font-mono text-xs text-neon-green mb-3 tracking-wider uppercase">
          Tools
        </h3>
        {toolItems.map((item) => (
          <NavItem key={item.label} {...item} />
        ))}
      </div>

      {/* AI ASSISTANT 组 */}
      <div className="px-3 py-4 border-t border-border-subtle">
        <h3 className="font-mono text-xs text-neon-green mb-3 tracking-wider uppercase">
          AI Assistant
        </h3>
        {aiItems.map((item) => (
          <NavItem key={item.label} {...item} />
        ))}
      </div>

      {/* 底部拖放提示 */}
      <div className="mt-auto p-4 border-t border-border-subtle">
        <div className="border-2 border-dashed border-border-default rounded-lg p-4
          text-center hover:border-neon-green hover:bg-neon-green/5 transition-all">
          <ArchiveIcon className="w-6 h-6 text-text-muted mx-auto mb-2" />
          <p className="font-mono text-xs text-text-muted">
            DRAG & DROP<br/>ARCHIVES HERE
          </p>
          <p className="font-mono text-[10px] text-text-muted mt-1">
            ZIP / RAR / 7Z / TAR / GZ / BZ2
          </p>
        </div>
      </div>
    </div>
  );
}

function NavItem({ label, count, icon: Icon }: {
  label: string;
  count?: number;
  icon: React.ComponentType<{ className?: string }>;
}) {
  const isActive = label === 'Analyze'; // 示例
  
  return (
    <button className={`w-full flex items-center gap-3 px-3 py-2 rounded-md
      font-mono text-sm transition-all mb-0.5
      ${isActive 
        ? 'bg-neon-green/10 text-neon-green border-l-2 border-neon-green' 
        : 'text-text-secondary hover:bg-bg-hover hover:text-text-primary'
      }`}
    >
      <Icon className={`w-4 h-4 ${isActive ? 'text-neon-green' : 'text-text-muted'}`} />
      <span className="flex-1 text-left">{label}</span>
      {count !== undefined && count > 0 && (
        <span className={`text-xs ${isActive ? 'text-neon-green' : 'text-text-muted'}`}>
          {count}
        </span>
      )}
    </button>
  );
}
```

### 3.5 底部状态栏 (Status Bar)

```tsx
// components/StatusBar.tsx
import React from 'react';

export default function StatusBar() {
  const stats = [
    { label: 'CPU', value: '12%', color: 'neon-green' },
    { label: 'MEM', value: '245MB', color: 'neon-blue' },
    { label: 'THREADS', value: '8', color: 'neon-orange' },
    { label: 'FILES/S', value: '134', color: 'neon-purple' },
    { label: 'SPEED', value: '12.4 MB/s', color: 'neon-green' },
    { label: 'RATIO', value: '72%', color: 'neon-green' },
  ];

  return (
    <div className="h-8 bg-bg-secondary border-t border-border-subtle 
      flex items-center px-4 gap-6">
      {stats.map((stat) => (
        <div key={stat.label} className="flex items-center gap-2">
          <span className="font-mono text-xs text-text-muted uppercase">
            {stat.label}
          </span>
          <span className={`font-mono text-xs text-${stat.color}`}>
            {stat.value}
          </span>
          {/* 迷你波形图 */}
          <div className="flex items-end gap-px h-3">
            {[3, 6, 4, 8, 5, 7, 4, 6, 3].map((h, i) => (
              <div 
                key={i}
                className={`w-0.5 bg-${stat.color} rounded-full`}
                style={{ 
                  height: `${h}px`,
                  opacity: 0.3 + (i / 20),
                }}
              />
            ))}
          </div>
        </div>
      ))}

      <div className="flex-1" />

      {/* 系统状态 */}
      <div className="flex items-center gap-2">
        <div className="w-2 h-2 rounded-full bg-neon-green animate-pulse" />
        <span className="font-mono text-xs text-neon-green tracking-wider">
          ALL SYSTEMS OPERATIONAL
        </span>
      </div>
    </div>
  );
}
```

### 3.6 按钮组件

```tsx
// components/ui/NeonButton.tsx
import React from 'react';

interface NeonButtonProps {
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost';
  size?: 'sm' | 'md' | 'lg';
  children: React.ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  icon?: React.ReactNode;
}

export default function NeonButton({
  variant = 'primary',
  size = 'md',
  children,
  onClick,
  disabled,
  icon,
}: NeonButtonProps) {
  const baseClasses = `
    font-mono font-medium rounded-md transition-all duration-300
    flex items-center justify-center gap-2
    disabled:opacity-50 disabled:cursor-not-allowed
  `;

  const sizeClasses = {
    sm: 'px-3 py-1.5 text-xs',
    md: 'px-4 py-2 text-sm',
    lg: 'px-6 py-3 text-sm',
  };

  const variantClasses = {
    primary: `
      bg-neon-green/10 border border-neon-green text-neon-green
      hover:bg-neon-green/20 hover:shadow-neon-green
      active:bg-neon-green/30
    `,
    secondary: `
      bg-bg-tertiary border border-border-default text-text-secondary
      hover:border-neon-green hover:text-neon-green hover:shadow-neon-green
    `,
    danger: `
      bg-neon-red/10 border border-neon-red text-neon-red
      hover:bg-neon-red/20 hover:shadow-neon-red
    `,
    ghost: `
      bg-transparent border border-transparent text-text-muted
      hover:text-text-secondary hover:bg-bg-hover
    `,
  };

  return (
    <button
      className={`${baseClasses} ${sizeClasses[size]} ${variantClasses[variant]}`}
      onClick={onClick}
      disabled={disabled}
    >
      {icon && <span className="w-4 h-4">{icon}</span>}
      {children}
    </button>
  );
}
```

### 3.7 输入框组件

```tsx
// components/ui/NeonInput.tsx
import React from 'react';

interface NeonInputProps {
  type?: string;
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
  label?: string;
  icon?: React.ReactNode;
  password?: boolean;
}

export default function NeonInput({
  type = 'text',
  placeholder,
  value,
  onChange,
  label,
  icon,
  password,
}: NeonInputProps) {
  return (
    <div className="flex flex-col gap-1.5">
      {label && (
        <label className="font-mono text-xs text-text-muted uppercase tracking-wider">
          {label}
        </label>
      )}
      <div className="relative">
        {icon && (
          <div className="absolute left-3 top-1/2 -translate-y-1/2 text-text-muted">
            {icon}
          </div>
        )}
        <input
          type={password ? 'password' : type}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          className={`
            w-full bg-bg-tertiary border border-border-default rounded-md
            font-mono text-sm text-text-primary placeholder:text-text-disabled
            focus:outline-none focus:border-neon-green focus:shadow-neon-green-sm
            transition-all duration-300
            ${icon ? 'pl-10' : 'pl-3'} pr-3 py-2
            ${password ? 'tracking-widest' : ''}
          `}
        />
        {password && (
          <button className="absolute right-3 top-1/2 -translate-y-1/2 text-text-muted
            hover:text-neon-green transition-colors">
            <EyeIcon className="w-4 h-4" />
          </button>
        )}
      </div>
    </div>
  );
}
```

---

## 4. 全局样式

```css
/* global.css */

/* 基础 */
* {
  box-sizing: border-box;
}

body {
  margin: 0;
  padding: 0;
  background: #0A0A0F;
  color: #FFFFFF;
  font-family: 'SF Pro Display', 'Segoe UI', system-ui, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

/* 自定义滚动条 */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: #111118;
}

::-webkit-scrollbar-thumb {
  background: #2D2D3A;
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: #3D3D4F;
}

/* 选中文本 */
::selection {
  background: rgba(0, 230, 118, 0.3);
  color: #FFFFFF;
}

/* 字体加载 */
@font-face {
  font-family: 'JetBrains Mono';
  src: url('/fonts/JetBrainsMono-Regular.woff2') format('woff2');
  font-weight: 400;
  font-display: swap;
}

@font-face {
  font-family: 'JetBrains Mono';
  src: url('/fonts/JetBrainsMono-Bold.woff2') format('woff2');
  font-weight: 700;
  font-display: swap;
}

/* 动画工具类 */
@keyframes glowPulse {
  0%, 100% { 
    box-shadow: 0 0 10px rgba(0, 230, 118, 0.4); 
  }
  50% { 
    box-shadow: 0 0 25px rgba(0, 230, 118, 0.6), 0 0 50px rgba(0, 230, 118, 0.2); 
  }
}

@keyframes progressShine {
  0% { background-position: -200% 0; }
  100% { background-position: 200% 0; }
}

@keyframes scanLine {
  0% { transform: translateY(-100%); }
  100% { transform: translateY(100vh); }
}

@keyframes float {
  0%, 100% { transform: translateY(0px); }
  50% { transform: translateY(-10px); }
}

/* 玻璃拟态效果 */
.glass {
  background: rgba(17, 17, 24, 0.7);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.05);
}

/* 网格背景 */
.grid-bg {
  background-image: 
    linear-gradient(rgba(0, 230, 118, 0.03) 1px, transparent 1px),
    linear-gradient(90deg, rgba(0, 230, 118, 0.03) 1px, transparent 1px);
  background-size: 50px 50px;
}

/* 渐变边框 */
.gradient-border {
  position: relative;
  background: #111118;
  border-radius: 8px;
}

.gradient-border::before {
  content: '';
  position: absolute;
  inset: -1px;
  border-radius: 9px;
  padding: 1px;
  background: linear-gradient(135deg, #00E676, #2962FF);
  -webkit-mask: 
    linear-gradient(#fff 0 0) content-box, 
    linear-gradient(#fff 0 0);
  -webkit-mask-composite: xor;
  mask-composite: exclude;
  pointer-events: none;
}
```

---

## 5. 效果速查表

| 效果 | 类名 / 用法 | 示例 |
|------|-------------|------|
| **霓虹绿文字** | `text-neon-green` | 标题、状态 |
| **霓虹绿发光** | `shadow-neon-green` | 按钮、卡片 |
| **脉冲发光** | `animate-glow-pulse` | 图标、状态指示 |
| **点阵背景** | `dot-grid-bg` | 主区域、拖放区 |
| **扫描线** | `scan-line-overlay` | 面板装饰 |
| **像素字体** | `font-pixel` | 大标题 |
| **等宽字体** | `font-mono` | 数据、代码 |
| **进度条光效** | `animate-progress-shine` | 进度条内层 |
| **玻璃效果** | `glass` | 弹窗、悬浮面板 |
| **渐变边框** | `gradient-border` | 特殊卡片 |
| **状态指示点** | `animate-pulse` + `bg-neon-green` | 在线状态 |
| **悬浮发光** | `hover:shadow-neon-green` | 交互元素 |
| **选中左边框** | `border-l-2 border-neon-green` | 导航项 |
| **标签/badge** | `bg-neon-green/10 border border-neon-green` | 版本标识 |

---

## 6. 关键实现要点

### 6.1 性能优化

- **will-change**: 对动画元素使用 `will-change: box-shadow` 或 `will-change: transform`
- **GPU 加速**: 使用 `transform: translateZ(0)` 或 `translate3d` 触发 GPU 渲染
- **减少重绘**: 避免在动画中改变 `width`，使用 `transform: scaleX` 替代
- **阴影优化**: 多层阴影会消耗性能，仅在 hover/激活状态使用完整阴影

### 6.2 深色模式适配

```css
/* 系统自动检测 */
@media (prefers-color-scheme: dark) {
  /* 默认就是暗色，无需额外处理 */
}

/* 减少动画偏好 */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

### 6.3 Tauri 特定适配

```css
/* Tauri 窗口无边框时的标题栏拖拽 */
.titlebar-drag {
  -webkit-app-region: drag;
}

.titlebar-no-drag {
  -webkit-app-region: no-drag;
}

/* Tauri 暗色主题 */
html {
  color-scheme: dark;
}
```

---

## 7. 设计原则总结

1. **深色为底，荧光为笔** — 背景永远深黑/深灰，信息用荧光色勾勒
2. **等宽字体即正义** — 所有数据、标签、按钮用等宽字体，强化科技感
3. **发光即反馈** — hover、激活、进度都用发光效果，不用传统变色
4. **点阵即空间** — 大面积空白区域用细点阵填充，避免空洞
5. **像素即灵魂** — 关键标题用像素字体，瞬间拉回复古终端感
6. **扫描线即呼吸** — 缓慢的扫描线动画让界面有"活着"的感觉
7. **状态即透明** — 底部状态栏、系统状态永远可见，绿色=正常

---

> 按照此指南，可以 1:1 还原设计图中的视觉效果。
