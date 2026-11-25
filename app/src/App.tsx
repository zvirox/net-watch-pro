import React, { useEffect } from 'react';
import { AlertTriangle } from 'lucide-react';
import {
    BreadcrumbBar,
    LinearButton,
    LinearCard,
    Sidebar,
    TitleBar,
    type BreadcrumbSegment,
    type NavScreenId,
} from '$lib/components/linear';
import { useDaemonStore } from '$lib/stores/useDaemonStore';
import { Dashboard } from '$lib/screens/Dashboard';
import { Live } from '$lib/screens/Live';
import { History } from '$lib/screens/History';
import { Apps } from '$lib/screens/Apps';
import { AppDetail } from '$lib/screens/AppDetail';
import { Diagnostics } from '$lib/screens/Diagnostics';
import { Settings } from '$lib/screens/Settings';
import { Upgrade } from '$lib/screens/Upgrade';
import { Onboarding } from '$lib/screens/Onboarding';

const SCREEN_TITLES: Record<string, string> = {
    dashboard: 'Dashboard',
    live: 'Live Traffic',
    history: 'History',
    apps: 'Apps',
    'app-detail': 'App Detail',
    diagnostics: 'Diagnostics',
    settings: 'Settings',
};

export const App: React.FC = () => {
    const {
        activeScreen,
        selectedAppId,
        appsList,
        sidebarCollapsed,
        connection,
        demoMode,
        onboardingDismissed,
        isPro,
        enablingMonitoring,
        setActiveScreen,
        toggleSidebar,
        openUpgrade,
        exitDemo,
        triggerEnableMonitoring,
        start,
    } = useDaemonStore();

    useEffect(() => {
        start();
    }, [start]);

    const showOnboarding =
        !demoMode &&
        !onboardingDismissed &&
        (connection === 'auth-required' || connection === 'auth-invalid');

    const daemonStoppedBanner =
        !demoMode &&
        onboardingDismissed &&
        connection !== 'connected' &&
        connection !== 'connecting';

    const selectedAppName =
        selectedAppId !== null
            ? (appsList.find((a) => a.id === selectedAppId)?.name ??
              `App #${String(selectedAppId)}`)
            : 'Detail';

    const breadcrumbSegments: BreadcrumbSegment[] = [
        { label: 'Net Watch Pro', onClick: () => setActiveScreen('dashboard') },
        ...(activeScreen === 'app-detail'
            ? [
                  { label: 'Apps', onClick: () => setActiveScreen('apps') },
                  { label: selectedAppName },
              ]
            : [{ label: SCREEN_TITLES[activeScreen] ?? 'Dashboard' }]),
    ];

    return (
        <div className="flex flex-col h-screen w-screen bg-[var(--bg-app)] text-[var(--text)] overflow-hidden">
            <TitleBar title="Net Watch Pro" />

            {showOnboarding ? (
                <Onboarding />
            ) : (
                <div className="flex flex-1 min-h-0 overflow-hidden">
                    <Sidebar
                        activeScreen={activeScreen}
                        collapsed={sidebarCollapsed}
                        isPro={isPro}
                        demoMode={demoMode}
                        onSelectScreen={(screen: NavScreenId) => setActiveScreen(screen)}
                        onToggleCollapse={toggleSidebar}
                        onOpenUpgrade={openUpgrade}
                    />

                    <main className="flex-1 flex flex-col min-w-0 overflow-hidden bg-[var(--bg-app)]">
                        <BreadcrumbBar
                            segments={breadcrumbSegments}
                            rightSlot={
                                demoMode ? (
                                    <div className="flex items-center gap-2">
                                        <span className="px-2 py-0.5 rounded bg-[var(--tint-blue)] text-[var(--tint-blue-fg)] text-xs font-medium">
                                            Demo mode
                                        </span>
                                        <LinearButton variant="ghost" size="sm" onClick={exitDemo}>
                                            Exit demo
                                        </LinearButton>
                                    </div>
                                ) : null
                            }
                        />

                        {daemonStoppedBanner && (
                            <div className="px-5 pt-4 shrink-0">
                                <LinearCard
                                    level={1}
                                    className="px-4 py-3 border-[var(--accent-amber)] bg-[var(--tint-amber)] flex flex-wrap items-center justify-between gap-3"
                                    role="alert"
                                >
                                    <div className="flex items-center gap-2 text-xs text-[var(--text)]">
                                        <AlertTriangle
                                            className="size-4 text-[var(--tint-amber-fg)] shrink-0"
                                            aria-hidden="true"
                                        />
                                        <span>
                                            Monitoring stopped — netwatchd is unreachable over the
                                            local Unix socket.
                                        </span>
                                    </div>
                                    <div className="flex items-center gap-2">
                                        <LinearButton
                                            variant="secondary"
                                            size="sm"
                                            onClick={() => setActiveScreen('diagnostics')}
                                        >
                                            Open diagnostics
                                        </LinearButton>
                                        <LinearButton
                                            variant="primary"
                                            size="sm"
                                            disabled={enablingMonitoring}
                                            onClick={() => void triggerEnableMonitoring()}
                                        >
                                            {enablingMonitoring ? 'Starting...' : 'Start daemon'}
                                        </LinearButton>
                                    </div>
                                </LinearCard>
                            </div>
                        )}

                        <div className="flex-1 overflow-y-auto p-5">
                            {activeScreen === 'dashboard' && <Dashboard />}
                            {activeScreen === 'live' && <Live />}
                            {activeScreen === 'history' && <History />}
                            {activeScreen === 'apps' && <Apps />}
                            {activeScreen === 'app-detail' && <AppDetail />}
                            {activeScreen === 'diagnostics' && <Diagnostics />}
                            {activeScreen === 'settings' && <Settings />}
                        </div>
                    </main>
                </div>
            )}

            <Upgrade />
        </div>
    );
};
