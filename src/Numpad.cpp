#include "Numpad.hpp"
#include "NumpadGraphics.h"

KeyboardKeycode Numpad::getChar(unsigned char row, unsigned char column) const {
    return this->mapping[row][column];
}

void Numpad::onPress(char row, char column) {
    const KeyboardKeycode _char = this->getChar(row, column);

    BootKeyboard.press(_char);
}

void Numpad::onRelease(char row, char column) {
    const KeyboardKeycode _char = this->getChar(row, column);
    this->drawNext = true;

    BootKeyboard.release(_char);
}

void Numpad::draw(U8G2* u8g2) {
    if(!this->drawNext) return;
    this->drawNext = false;

    const bool state = BootKeyboard.getLeds() & LED_NUM_LOCK;
    const Sprite* sprite = &numpadSprites[state ? 0 : 1];

    u8g2->clearBuffer();
    u8g2->drawXBMP(4, 1, sprite->width, sprite->height, sprite->data);    
    // u8g2->setFont(u8g2_font_sticker_mel_tr);
    // u8g2->drawStr(38,24,"Love You!!");
    u8g2->sendBuffer();
}

void Numpad::tick(const unsigned long ms) {
    const unsigned char leds = BootKeyboard.getLeds();

    this->drawNext |= this->ledState != leds;
    this->ledState = leds;
}

void Numpad::onShow() {
    this->drawNext = true;
}