import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Link, useLocation, useNavigate } from "react-router-dom";
import { 
  LayoutDashboard, 
  BookOpen, 
  Brain, 
  Settings, 
  ChevronLeft,
  GraduationCap,
  Flame,
  Sparkles,
  LogOut
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { useAuth } from "@/contexts/AuthContext";

const navItems = [
  { path: '/', icon: LayoutDashboard, label: 'Dashboard' },
  { path: '/learn', icon: BookOpen, label: 'Learn' },
  { path: '/quiz', icon: Brain, label: 'Quiz' },
  { path: '/settings', icon: Settings, label: 'Settings' },
];

export function Sidebar() {
  const [isCollapsed, setIsCollapsed] = useState(false);
  const location = useLocation();
  const navigate = useNavigate();
  const { user, logout } = useAuth();

  return (
    <motion.aside
      initial={false}
      animate={{ width: isCollapsed ? 72 : 260 }}
      transition={{ type: "spring", stiffness: 300, damping: 30 }}
      className="relative h-screen border-r border-sidebar-border bg-sidebar flex flex-col overflow-hidden"
    >
      {/* Subtle gradient overlay */}
      <div className="absolute inset-0 bg-gradient-to-b from-sidebar via-sidebar to-sidebar/90 pointer-events-none" />
      
      {/* Logo */}
      <div className="relative z-10 flex items-center gap-3 px-4 h-16 border-b border-sidebar-border">
        <motion.div 
          whileHover={{ scale: 1.05, rotate: 2 }}
          whileTap={{ scale: 0.95 }}
          className="flex items-center justify-center h-10 w-10 rounded-xl gradient-primary shadow-md"
        >
          <GraduationCap className="h-5 w-5 text-primary-foreground" />
        </motion.div>
        <AnimatePresence mode="wait">
          {!isCollapsed && (
            <motion.div
              initial={{ opacity: 0, x: -10 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -10 }}
              transition={{ duration: 0.2 }}
              className="flex flex-col"
            >
              <span className="font-display font-bold text-lg text-sidebar-foreground tracking-tight">
                Study Buddy
              </span>
              <span className="text-[10px] text-muted-foreground font-medium tracking-wide uppercase">
                Learn smarter
              </span>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Streak badge */}
      <AnimatePresence mode="wait">
        {!isCollapsed && (
          <motion.div 
            initial={{ opacity: 0, y: -10, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -10, scale: 0.95 }}
            transition={{ duration: 0.2, delay: 0.1 }}
            className="relative z-10 mx-3 mt-4"
          >
            <div className="relative overflow-hidden p-4 rounded-xl bg-gradient-to-br from-accent/15 via-accent/10 to-accent/5 border border-accent/20">
              {/* Shimmer effect */}
              <div className="absolute inset-0 shimmer" />
              
              <div className="relative flex items-center gap-3">
                <motion.div
                  animate={{ 
                    scale: [1, 1.1, 1],
                  }}
                  transition={{ 
                    duration: 2, 
                    repeat: Infinity,
                    ease: "easeInOut"
                  }}
                  className="flex items-center justify-center h-10 w-10 rounded-xl bg-accent/20"
                >
                  <Flame className="h-5 w-5 text-accent" />
                </motion.div>
                <div>
                  <div className="flex items-center gap-1.5">
                    <span className="text-lg font-display font-bold text-foreground">7</span>
                    <span className="text-sm font-medium text-foreground">day streak!</span>
                  </div>
                  <p className="text-xs text-muted-foreground flex items-center gap-1">
                    <Sparkles className="h-3 w-3 text-accent" />
                    Keep the momentum
                  </p>
                </div>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Collapsed streak indicator */}
      <AnimatePresence mode="wait">
        {isCollapsed && (
          <motion.div
            initial={{ opacity: 0, scale: 0.8 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.8 }}
            className="relative z-10 mx-auto mt-4"
          >
            <motion.div
              animate={{ scale: [1, 1.05, 1] }}
              transition={{ duration: 2, repeat: Infinity }}
              className="flex items-center justify-center h-10 w-10 rounded-xl bg-accent/15 border border-accent/20"
            >
              <Flame className="h-5 w-5 text-accent" />
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Navigation */}
      <nav className="relative z-10 flex-1 px-3 py-6 space-y-1">
        {navItems.map((item, index) => {
          const isActive = location.pathname === item.path;
          return (
            <Link key={item.path} to={item.path}>
              <motion.div
                initial={false}
                whileHover={{ x: 4 }}
                whileTap={{ scale: 0.98 }}
                transition={{ type: "spring", stiffness: 400, damping: 25 }}
                className={cn(
                  "relative flex items-center gap-3 px-3 py-3 rounded-xl transition-all duration-200",
                  isActive 
                    ? "bg-primary text-primary-foreground shadow-md" 
                    : "text-sidebar-foreground hover:bg-muted/80"
                )}
              >
                {/* Active indicator glow */}
                {isActive && (
                  <motion.div
                    layoutId="activeGlow"
                    className="absolute inset-0 rounded-xl bg-primary shadow-lg"
                    style={{ 
                      boxShadow: '0 4px 15px -3px hsl(var(--primary) / 0.4)'
                    }}
                    transition={{ type: "spring", stiffness: 350, damping: 30 }}
                  />
                )}
                
                <motion.div
                  className="relative z-10"
                  animate={isActive ? { rotate: [0, -5, 5, 0] } : {}}
                  transition={{ duration: 0.4 }}
                >
                  <item.icon className={cn(
                    "h-5 w-5 transition-colors",
                    isActive ? "text-primary-foreground" : "text-muted-foreground"
                  )} />
                </motion.div>
                
                <AnimatePresence mode="wait">
                  {!isCollapsed && (
                    <motion.span 
                      initial={{ opacity: 0, x: -10 }}
                      animate={{ opacity: 1, x: 0 }}
                      exit={{ opacity: 0, x: -10 }}
                      transition={{ duration: 0.15, delay: index * 0.02 }}
                      className={cn(
                        "relative z-10 text-sm font-medium",
                        isActive ? "text-primary-foreground" : ""
                      )}
                    >
                      {item.label}
                    </motion.span>
                  )}
                </AnimatePresence>
              </motion.div>
            </Link>
          );
        })}
      </nav>

      {/* User section */}
      <div className="relative z-10 px-3 pb-4 space-y-2">
        <div className={cn(
          "flex items-center gap-3 p-3 rounded-xl bg-muted/50 border border-border/50",
          isCollapsed && "justify-center p-2"
        )}>
          <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground text-xs font-bold">
            {user?.displayName?.charAt(0) ?? "?"}
          </div>
          <AnimatePresence mode="wait">
            {!isCollapsed && (
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="flex-1 min-w-0"
              >
                <p className="text-sm font-medium text-foreground truncate">{user?.displayName}</p>
                <p className="text-[10px] text-muted-foreground truncate">{user?.email}</p>
              </motion.div>
            )}
          </AnimatePresence>
          <AnimatePresence mode="wait">
            {!isCollapsed && (
              <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-7 w-7 text-muted-foreground hover:text-destructive"
                  onClick={() => { logout(); navigate("/auth"); }}
                >
                  <LogOut className="h-3.5 w-3.5" />
                </Button>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>

      {/* Collapse toggle */}
      <motion.div
        initial={false}
        animate={{ rotate: isCollapsed ? 180 : 0 }}
        transition={{ duration: 0.2 }}
        className="absolute -right-3 top-20 z-20"
      >
        <Button
          variant="outline"
          size="icon"
          onClick={() => setIsCollapsed(!isCollapsed)}
          className="h-6 w-6 rounded-full border border-border bg-background shadow-md hover:bg-muted hover:shadow-lg transition-all duration-200"
        >
          <ChevronLeft className="h-3 w-3" />
        </Button>
      </motion.div>
    </motion.aside>
  );
}