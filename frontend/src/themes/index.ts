import './dark-academia.css';
import './cyberpunk.css';
import './parchment.css';
import './cosmic.css';
import './noir.css';
import './nordic.css';
import './glassmorphism.css';
import './film-grain.css';
import './verdant-grove.css';
import './art-deco.css';
import './knolling.css';
import './industrial.css';
import './streamline-moderne.css';
import './pixel-art.css';
import './circuit-board.css';
import './byzantine.css';
import './editorial.css';
import './persian-miniature.css';
import './concrete-brutalist.css';
import './windows31.css';
import './retro-terminal.css';
import './swiss-international.css';
import './air-traffic-control.css';
import './japanese-zen.css';
import './de-stijl.css';
import './constructivism.css';
import './letterpress.css';
import './risograph.css';
import './art-nouveau.css';
import './frutiger-aero.css';

export const themes = [
  { id: 'default', name: 'Default', className: '', accent: '#6366f1', font: 'system-ui, sans-serif', bg: '#ffffff' },
  { id: 'dark-academia', name: 'Dark Academia', className: 'theme-dark-academia', accent: '#b8860b', font: "'Cormorant Garamond', serif", bg: '#2a1a0a' },
  { id: 'cyberpunk', name: 'Cyberpunk', className: 'theme-cyberpunk', accent: '#05d9e8', font: "'Orbitron', sans-serif", bg: '#0a0a0f' },
  { id: 'parchment', name: 'Parchment', className: 'theme-parchment', accent: '#c53a27', font: "'Cinzel', serif", bg: '#e8d5a3' },
  { id: 'cosmic', name: 'Cosmic', className: 'theme-cosmic', accent: '#40e0d0', font: "'Cormorant Garamond', serif", bg: '#030108' },
  { id: 'noir', name: 'Noir', className: 'theme-noir', accent: '#ff6b35', font: "'Playfair Display', serif", bg: '#0a0a0a' },
  { id: 'nordic', name: 'Nordic', className: 'theme-nordic', accent: '#c67b5c', font: "'Playfair Display', serif", bg: '#fafbfc' },
  { id: 'glassmorphism', name: 'Glassmorphism', className: 'theme-glassmorphism', accent: '#667eea', font: "'Inter', sans-serif", bg: '#0f0c29' },
  { id: 'film-grain', name: 'Film Grain', className: 'theme-film-grain', accent: '#d4a066', font: "'Special Elite', monospace", bg: '#faf6ed' },
  { id: 'verdant-grove', name: 'Verdant Grove', className: 'theme-verdant-grove', accent: '#4f8a6e', font: "'Cormorant Garamond', serif", bg: '#f4f8f3' },
  { id: 'art-deco', name: 'Art Deco', className: 'theme-art-deco', accent: '#d4af37', font: "'Poiret One', cursive", bg: '#0d0d0d' },
  { id: 'knolling', name: 'Knolling', className: 'theme-knolling', accent: '#c4572a', font: "'IBM Plex Sans', sans-serif", bg: '#fafaf8' },
  { id: 'industrial', name: 'Industrial', className: 'theme-industrial', accent: '#ffb800', font: "'Oswald', sans-serif", bg: '#1a1d1f' },
  { id: 'streamline-moderne', name: 'Streamline Moderne', className: 'theme-streamline-moderne', accent: '#7ebdb4', font: "'Quicksand', sans-serif", bg: '#0f1a2e' },
  { id: 'pixel-art', name: 'Pixel Art', className: 'theme-pixel-art', accent: '#00d4ff', font: "'Press Start 2P', monospace", bg: '#1a1a2e' },
  { id: 'circuit-board', name: 'Circuit Board', className: 'theme-circuit-board', accent: '#c87533', font: "'JetBrains Mono', monospace", bg: '#0a3d0a' },
  { id: 'byzantine', name: 'Byzantine', className: 'theme-byzantine', accent: '#d4a012', font: "'Cinzel', serif", bg: '#1a0a2e' },
  { id: 'editorial', name: 'Editorial', className: 'theme-editorial', accent: '#c41e3a', font: "'Playfair Display', serif", bg: '#faf8f5' },
  { id: 'persian-miniature', name: 'Persian Miniature', className: 'theme-persian-miniature', accent: '#d4a012', font: "'Cinzel Decorative', serif", bg: '#f5f0e1' },
  { id: 'concrete-brutalist', name: 'Concrete Brutalist', className: 'theme-concrete-brutalist', accent: '#ffb800', font: "'Bebas Neue', sans-serif", bg: '#4a4a4a' },
  { id: 'windows31', name: 'Windows 3.1', className: 'theme-windows31', accent: '#000080', font: "'Segoe UI', Tahoma, sans-serif", bg: '#c0c0c0' },
  { id: 'retro-terminal', name: 'Retro Terminal', className: 'theme-retro-terminal', accent: '#00ff00', font: "'VT323', monospace", bg: '#000a00' },
  { id: 'swiss-international', name: 'Swiss International', className: 'theme-swiss-international', accent: '#E30613', font: "'Inter', sans-serif", bg: '#ffffff' },
  { id: 'air-traffic-control', name: 'Air Traffic Control', className: 'theme-air-traffic-control', accent: '#00ff66', font: "'Orbitron', sans-serif", bg: '#0a0f0a' },
  { id: 'japanese-zen', name: 'Japanese Zen', className: 'theme-japanese-zen', accent: '#8B9A6B', font: "'Noto Serif JP', serif", bg: '#F5F2EB' },
  { id: 'de-stijl', name: 'De Stijl', className: 'theme-de-stijl', accent: '#D40920', font: "'Work Sans', sans-serif", bg: '#ffffff' },
  { id: 'constructivism', name: 'Constructivism', className: 'theme-constructivism', accent: '#CC0000', font: "'Bebas Neue', sans-serif", bg: '#F5F5F0' },
  { id: 'letterpress', name: 'Letterpress', className: 'theme-letterpress', accent: '#8B0000', font: "'Playfair Display', serif", bg: '#F5ECD7' },
  { id: 'risograph', name: 'Risograph', className: 'theme-risograph', accent: '#FF6B6B', font: "'Space Mono', monospace", bg: '#FDF6E3' },
  { id: 'art-nouveau', name: 'Art Nouveau', className: 'theme-art-nouveau', accent: '#C9A227', font: "'Playfair Display', serif", bg: '#F5F0E6' },
  { id: 'frutiger-aero', name: 'Frutiger Aero', className: 'theme-frutiger-aero', accent: '#3a9fd5', font: "'Source Sans Pro', sans-serif", bg: '#e8f4fc' },
] as const;

export type ThemeId = typeof themes[number]['id'];
