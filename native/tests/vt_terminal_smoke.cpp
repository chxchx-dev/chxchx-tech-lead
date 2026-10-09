#include "../src/integrations/vt_terminal_widget.hpp"

#include <QApplication>
#include <QImage>
#include <QKeyEvent>

namespace {

QImage render(VtTerminalWidget &terminal)
{
    QImage image(terminal.size(), QImage::Format_ARGB32_Premultiplied);
    image.fill(Qt::transparent);
    terminal.render(&image);
    return image;
}

void press(VtTerminalWidget &terminal, int key, Qt::KeyboardModifiers modifiers)
{
    QKeyEvent event(QEvent::KeyPress, key, modifiers);
    QApplication::sendEvent(&terminal, &event);
    QCoreApplication::processEvents();
}

} // namespace

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
    VtTerminalWidget terminal;
    terminal.resize(720, 440);
    terminal.show();
    QCoreApplication::processEvents();

    QByteArray output = "\033[31mANSI-RED\033[0m\r\n";
    for (int line = 0; line < 48; ++line)
        output.append("history-line-").append(QByteArray::number(line)).append("\r\n");
    terminal.feed(output);
    QCoreApplication::processEvents();

    const QImage liveView = render(terminal);
    press(terminal, Qt::Key_Home, Qt::ControlModifier);
    const QImage oldestView = render(terminal);
    if (oldestView == liveView) return 1;

    press(terminal, Qt::Key_PageDown, Qt::ShiftModifier);
    const QImage laterHistoryView = render(terminal);
    if (laterHistoryView == oldestView || laterHistoryView == liveView) return 2;

    press(terminal, Qt::Key_End, Qt::ControlModifier);
    if (render(terminal) != liveView) return 3;

    bool foundRedAnsiPixel = false;
    for (int y = 0; y < oldestView.height() && !foundRedAnsiPixel; ++y) {
        for (int x = 0; x < oldestView.width(); ++x) {
            const QColor pixel = oldestView.pixelColor(x, y);
            if (pixel.red() > 100 && pixel.green() < 80 && pixel.blue() < 80) {
                foundRedAnsiPixel = true;
                break;
            }
        }
    }
    if (!foundRedAnsiPixel) return 4;

    int resizedColumns = 0;
    int resizedRows = 0;
    QObject::connect(&terminal, &VtTerminalWidget::terminalResized, &app,
        [&resizedColumns, &resizedRows](int columns, int rows) {
            resizedColumns = columns;
            resizedRows = rows;
        });
    terminal.resize(480, 320);
    QCoreApplication::processEvents();
    if (resizedColumns <= 0 || resizedRows <= 0) return 5;
    const QImage resizedLiveView = render(terminal);
    press(terminal, Qt::Key_Home, Qt::ControlModifier);
    if (render(terminal) == resizedLiveView) return 6;
    press(terminal, Qt::Key_End, Qt::ControlModifier);
    return 0;
}
