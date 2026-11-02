#pragma once

#include <QString>

// Stylesheet for the visual chat, built on Studio's navy/cyan tokens
// (see studio_theme.hpp). Scoped by object names so it never leaks.
inline QString agentChatStyleSheet()
{
    return QStringLiteral(R"(
        #agentChat, #chatContent, #chatView, #chatWelcome, #chatFooter { background-color: #0b1220; }
        #chatHeader QLabel, #usageBar, #chatSettings, #composer QLabel { background: transparent; }
        #agentChat QScrollBar:vertical { background: transparent; width: 9px; margin: 2px; }
        #agentChat QScrollBar:horizontal { background: transparent; height: 9px; margin: 2px; }
        #agentChat QScrollBar::handle { background: #2b405b; border-radius: 3px; min-height: 24px; min-width: 24px; }
        #agentChat QScrollBar::handle:hover { background: #3b5578; }
        #agentChat QScrollBar::add-line, #agentChat QScrollBar::sub-line { width: 0; height: 0; border: 0; }
        #agentChat QScrollBar::add-page, #agentChat QScrollBar::sub-page { background: transparent; }
        #chatHeader { background-color: #0e1728; border-bottom: 1px solid #1d2b40; }
        #chatTitle { color: #e6edf7; font-size: 15px; font-weight: 700; }
        #chatSubtitle { color: #91a7c2; font-size: 12px; }
        #chatHeaderButton { background-color: transparent; border: 1px solid #263a55; border-radius: 14px;
            padding: 5px 12px; color: #dce8f7; }
        #chatHeaderButton:hover { border-color: #20c5d4; color: #8cf2f4; }

        #usagePill { background-color: #101c2e; border: 1px solid #1d2b40; border-radius: 11px; }
        #usageLabel { color: #91a7c2; font-size: 11px; }
        #usageValue { color: #dce8f7; font-size: 11px; font-weight: 600; }
        #usageSpend { color: #91a7c2; font-size: 11px; }
        #usageMeter { background-color: #1d2b40; border: 0; border-radius: 2px; }
        #usageMeter::chunk { background-color: #42be65; border-radius: 2px; }
        #usageMeter[level="medium"]::chunk { background-color: #f1c21b; }
        #usageMeter[level="high"]::chunk { background-color: #fa4d56; }

        #welcomeTitle { color: #e6edf7; font-size: 20px; font-weight: 700; }
        #welcomeSubtitle { color: #91a7c2; font-size: 13px; }
        #suggestionChip { background-color: #101c2e; border: 1px solid #263a55; border-radius: 12px;
            padding: 10px 14px; color: #dce8f7; text-align: left; min-width: 230px; }
        #suggestionChip:hover { background-color: #14283c; border-color: #20c5d4; color: #8cf2f4; }

        #userBubble { background-color: #123846; border: 1px solid #24566a; border-radius: 14px; }
        #userText { color: #e6edf7; font-size: 13px; background: transparent; }
        #chatThumb { border: 1px solid #24566a; border-radius: 6px; background: transparent; }
        #agentName { font-weight: 700; font-size: 12px; background: transparent; }
        #agentText { color: #dce8f7; font-size: 13px; background: transparent; }
        #reasoningText, #noticeText { color: #91a7c2; font-size: 12px; background: transparent; }
        #reasoningText { font-style: italic; padding: 2px 0 4px 10px; border-left: 2px solid #263a55; }
        #typingLabel { color: #91a7c2; font-size: 12px; background: transparent; }

        #codeBlock { background-color: #08101d; border: 1px solid #1d2b40; border-radius: 10px; }
        #codeHeader { background-color: #0e1728; border-bottom: 1px solid #1d2b40;
            border-top-left-radius: 10px; border-top-right-radius: 10px; }
        #codeLanguage { color: #91a7c2; font-size: 11px; background: transparent; }
        #chatMono { background-color: #08101d; color: #c9d7ea; border: 0; padding: 6px 10px;
            border-bottom-left-radius: 10px; border-bottom-right-radius: 10px; }

        #toolCard { background-color: #0e1728; border: 1px solid #1d2b40; border-left: 3px solid #20c5d4;
            border-radius: 8px; }
        #toolCard[status="completed"] { border-left-color: #42be65; }
        #toolCard[status="failed"] { border-left-color: #fa4d56; }
        #toolTitle { color: #c9d7ea; background: transparent; }
        #toolMeta { color: #64748b; font-size: 11px; background: transparent; }

        #chatGhost { background-color: transparent; border: 0; color: #91a7c2; padding: 2px 6px; }
        #chatGhost:hover { color: #8cf2f4; background-color: #15263c; border-radius: 6px; }

        #errorCard { background-color: #2a1218; border: 1px solid #5c2330; border-radius: 12px; }
        #errorTitle { color: #ff8389; font-weight: 700; background: transparent; }
        #errorText { color: #f2c4c8; background: transparent; }
        #commandBox { background-color: #08101d; border: 1px solid #263a55; border-radius: 8px; }
        #commandText { color: #8cf2f4; background: transparent; }

        #composer { background-color: #0e1728; border: 1px solid #263a55; border-radius: 14px; }
        #composer[focused="true"] { border-color: #20c5d4; }
        #composerInput { background-color: transparent; border: 0; color: #e6edf7; font-size: 13px; padding: 4px; }
        #composerHint { color: #64748b; font-size: 11px; background: transparent; }
        #attachmentChip { background-color: #123846; border: 1px solid #24566a; border-radius: 10px;
            color: #dce8f7; padding: 3px 8px; }
        #composerIcon { background-color: transparent; border: 0; border-radius: 15px; font-size: 15px;
            min-width: 30px; min-height: 30px; padding: 0; }
        #composerIcon:hover { background-color: #17243a; }
        #pillPicker { background-color: #101c2e; border: 1px solid #263a55; border-radius: 12px;
            padding: 4px 11px; color: #dce8f7; font-size: 12px; }
        #pillPicker:hover { border-color: #20c5d4; color: #8cf2f4; }
        #pillPicker::menu-indicator { image: none; width: 0; }
        #pillMenu { background-color: #101c2e; color: #dce8f7; border: 1px solid #263a55; border-radius: 8px;
            padding: 4px; }
        #pillMenu::item { padding: 6px 22px 6px 10px; border-radius: 6px; }
        #pillMenu::item:selected { background-color: #123846; color: #8cf2f4; }
        #sendButton { background-color: #087e8b; border: 1px solid #21c6d5; border-radius: 17px; color: #ffffff;
            font-size: 15px; font-weight: 700; min-width: 34px; max-width: 34px; min-height: 34px; max-height: 34px;
            padding: 0; }
        #sendButton:hover { background-color: #0b97a6; }
        #sendButton[running="true"] { background-color: #743846; border-color: #fa4d56; }
        #sendButton:disabled { background-color: #111a29; border-color: #1d2b40; color: #64748b; }
    )");
}
