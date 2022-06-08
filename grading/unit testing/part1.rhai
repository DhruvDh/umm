// https://www.dropbox.com/s/4ygey1q3gqxqpf9/part1.rhai?raw=1

let project = new_project();

let req_1 = grade_by_tests(
    [("ConcertTicket.WalletTest")],
    [
        "ConcertTicket.WalletTest#testAdd",
        "ConcertTicket.WalletTest#testRemove",
        "ConcertTicket.WalletTest#testResize",
        "ConcertTicket.WalletTest#testGetSize",
        "ConcertTicket.WalletTest#testGetLength",
        "ConcertTicket.WalletTest#testToString",
    ],
    project,
    25.0,
    "1",
);

show_results([req_1]);