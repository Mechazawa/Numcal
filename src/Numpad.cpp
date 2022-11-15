#include "Numpad.hpp"

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

    u8g2->clearBuffer();
    u8g2->setFont(u8g2_font_ncenB08_tr);	// choose a suitable font
    
    if(BootKeyboard.getLeds() & LED_NUM_LOCK) {
        u8g2->drawStr(2,10,"Numlock ON");
    } else {
        u8g2->drawStr(2,10,"Numlock OFF");
    }
    u8g2->sendBuffer();
}

void Numpad::tick(const unsigned long ms) {
    const unsigned char leds = BootKeyboard.getLeds();

    this->drawNext = this->ledState != leds;
    this->ledState = leds;
}

void Numpad::onShow() {
    this->drawNext = true;
}