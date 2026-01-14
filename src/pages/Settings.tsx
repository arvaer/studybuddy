import { motion } from "framer-motion";
import { 
  User, 
  Bell, 
  Moon, 
  Clock, 
  BookOpen,
  Target,
  Zap
} from "lucide-react";
import { AppLayout } from "@/components/layout/app-layout";
import { Card } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Slider } from "@/components/ui/slider";
import { Label } from "@/components/ui/label";

export default function SettingsPage() {
  return (
    <AppLayout>
      <div className="p-8 max-w-2xl mx-auto">
        <motion.header 
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          className="mb-8"
        >
          <h1 className="font-display text-3xl font-bold text-foreground tracking-tight">
            Settings
          </h1>
          <p className="mt-1 text-muted-foreground">
            Customize your learning experience
          </p>
        </motion.header>

        <div className="space-y-6">
          {/* Profile */}
          <Card className="p-6">
            <div className="flex items-center gap-3 mb-6">
              <User className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display font-semibold text-foreground">Profile</h2>
            </div>
            <div className="flex items-center gap-4">
              <div className="h-16 w-16 rounded-full bg-primary/10 flex items-center justify-center">
                <span className="font-display text-xl font-bold text-primary">SB</span>
              </div>
              <div>
                <p className="font-medium text-foreground">Study Buddy User</p>
                <p className="text-sm text-muted-foreground">student@example.com</p>
              </div>
            </div>
          </Card>

          {/* Learning Preferences */}
          <Card className="p-6">
            <div className="flex items-center gap-3 mb-6">
              <BookOpen className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display font-semibold text-foreground">Learning Preferences</h2>
            </div>
            
            <div className="space-y-6">
              <div className="flex items-center justify-between">
                <div>
                  <Label className="text-foreground">Reinforcement Prompts</Label>
                  <p className="text-sm text-muted-foreground">
                    Show quick check questions during reading
                  </p>
                </div>
                <Switch defaultChecked />
              </div>

              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <Label className="text-foreground">Question Frequency</Label>
                  <span className="text-sm text-muted-foreground">Medium</span>
                </div>
                <Slider defaultValue={[50]} max={100} step={25} />
                <div className="flex justify-between text-xs text-muted-foreground">
                  <span>Fewer</span>
                  <span>More</span>
                </div>
              </div>

              <div className="flex items-center justify-between">
                <div>
                  <Label className="text-foreground">AI-Generated Notes</Label>
                  <p className="text-sm text-muted-foreground">
                    Automatically create study notes
                  </p>
                </div>
                <Switch defaultChecked />
              </div>
            </div>
          </Card>

          {/* Goals */}
          <Card className="p-6">
            <div className="flex items-center gap-3 mb-6">
              <Target className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display font-semibold text-foreground">Daily Goals</h2>
            </div>
            
            <div className="space-y-6">
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <Label className="text-foreground">Study Time Goal</Label>
                  <span className="text-sm text-muted-foreground">30 minutes</span>
                </div>
                <Slider defaultValue={[30]} max={120} step={15} />
              </div>

              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <Label className="text-foreground">Daily Questions</Label>
                  <span className="text-sm text-muted-foreground">10 questions</span>
                </div>
                <Slider defaultValue={[10]} max={50} step={5} />
              </div>
            </div>
          </Card>

          {/* Notifications */}
          <Card className="p-6">
            <div className="flex items-center gap-3 mb-6">
              <Bell className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display font-semibold text-foreground">Notifications</h2>
            </div>
            
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <Label className="text-foreground">Daily Reminders</Label>
                  <p className="text-sm text-muted-foreground">
                    Get reminded to study
                  </p>
                </div>
                <Switch defaultChecked />
              </div>

              <div className="flex items-center justify-between">
                <div>
                  <Label className="text-foreground">Streak Alerts</Label>
                  <p className="text-sm text-muted-foreground">
                    Don't lose your streak!
                  </p>
                </div>
                <Switch defaultChecked />
              </div>

              <div className="flex items-center justify-between">
                <div>
                  <Label className="text-foreground">Review Reminders</Label>
                  <p className="text-sm text-muted-foreground">
                    When concepts need reinforcement
                  </p>
                </div>
                <Switch />
              </div>
            </div>
          </Card>

          {/* Appearance */}
          <Card className="p-6">
            <div className="flex items-center gap-3 mb-6">
              <Moon className="h-5 w-5 text-muted-foreground" />
              <h2 className="font-display font-semibold text-foreground">Appearance</h2>
            </div>
            
            <div className="flex items-center justify-between">
              <div>
                <Label className="text-foreground">Reduce Animations</Label>
                <p className="text-sm text-muted-foreground">
                  For accessibility
                </p>
              </div>
              <Switch />
            </div>
          </Card>
        </div>
      </div>
    </AppLayout>
  );
}
