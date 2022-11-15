#include "Numpad.hpp"

KeyboardKeycode Numpad::getChar(unsigned char row, unsigned char column) const {
    return this->mapping[this->numlock][row][column];
}

void Numpad::onPress(char row, char column) {
    KeyboardKeycode _char = this->getChar(row, column);

    NKROKeyboard.press(_char);
    NKROKeyboard.send();

    if (_char == KEY_NUM_LOCK) {
        this->numlock = !this->numlock;
    }

    this->drawNext = true;
}

void Numpad::onRelease(char row, char column) {
    this->drawNext = true;

    NKROKeyboard.release(this->getChar(row, column));
    NKROKeyboard.send();
}

void Numpad::draw(U8G2* u8g2) {
    if(!this->drawNext) return;
    this->drawNext = false;

    u8g2->clearBuffer();
    u8g2->setFont(u8g2_font_ncenB08_tr);	// choose a suitable font
    
    if(this->numlock) {
        u8g2->drawStr(2,10,"Numlock ON");
    } else {
        u8g2->drawStr(2,10,"Numlock OFF");
    }
    u8g2->sendBuffer();
}

void Numpad::onShow() {
    this->drawNext = true;
}