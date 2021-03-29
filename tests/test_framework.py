from requests import get, post, put, delete

DOMAIN = "http://127.0.0.1:8080"


def test_404():
    """
    Checks that a non existent route returns 404
    """
    assert get(f"{DOMAIN}/404").status_code == 404


def test_200():
    """
    Checks that a route that exists and responds returns 200 OK
    """
    assert get(f"{DOMAIN}/200").status_code == 200


def test_validate_ok():
    """
    Checks that validators validate correctly.
    """
    r = get(f"{DOMAIN}/numbers/1234")
    assert r.status_code == 200


def test_validate_404():
    """
    Checks that validators that do not match return 404
    """
    r = get(f"{DOMAIN}/numbers/1234a")
    assert r.status_code == 404


def test_get():
    """
    Checks that a get request returns 200 on a GET only ep and
    returns 405 for method not allowed on invalid methods.
    """
    r = get(f"{DOMAIN}/get")
    assert r.status_code == 200

    r = post(f"{DOMAIN}/get")
    assert r.status_code == 405


def test_post():
    """
    Checks that a post request returns 200 on a POST only ep and
    returns 405 for method not allowed on invalid methods.
    """
    r = post(f"{DOMAIN}/post")
    assert r.status_code == 200

    r = get(f"{DOMAIN}/post")
    assert r.status_code == 405


def test_put():
    """
    Checks that a put request returns 200 on a PUT only ep and
    returns 405 for method not allowed on invalid methods.
    """
    r = put(f"{DOMAIN}/put")
    assert r.status_code == 200

    r = delete(f"{DOMAIN}/put")
    assert r.status_code == 405


def test_delete():
    """
    Checks that a delete request returns 200 on a DELETE only ep and
    returns 405 for method not allowed on invalid methods.
    """
    r = delete(f"{DOMAIN}/delete")
    assert r.status_code == 200

    r = put(f"{DOMAIN}/delete")
    assert r.status_code == 405
