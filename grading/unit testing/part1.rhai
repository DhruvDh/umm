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