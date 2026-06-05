#include <iostream>

// Test 1: llhttp — HTTP parser
#include <llhttp.h>

// Test 2: ryu — fast float-to-string
#include <ryu.h>

int main() {
    llhttp_t parser;
    llhttp_settings_t settings;
    llhttp_settings_init(&settings);
    llhttp_init(&parser, HTTP_BOTH, &settings);
    std::cout << "llhttp: OK\n";

    char buf[64];
    ryu_d2s_buffered(3.14159, buf);
    std::cout << "ryu: OK (" << buf << ")\n";

    return 0;
}
