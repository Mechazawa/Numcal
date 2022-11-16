#include "KeyboardInterface.hpp"

#pragma once


class Numpad : public KeyboardInterface {
    private:
        int x; // dino height
        
    protected:
        char getAction(char row, char column) const;

    public:
        void onPress(char row, char column) override;
        void draw(U8G2* u8g2) override;
        void tick(const unsigned long ms) override;
        void onShow() override;
};