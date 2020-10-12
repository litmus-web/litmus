import asyncio


class RustProtocol:
    """Interface for stream protocol.

        The user should implement this interface.  They can inherit from
        this class but don't need to.  The implementations here do
        nothing (they don't raise exceptions).

        When the user wants to requests a transport, they pass a protocol
        factory to a utility function (e.g., EventLoop.create_connection()).

        When the connection is made successfully, connection_made() is
        called with a suitable transport object.  Then data_received()
        will be called 0 or more times with data (bytes) received from the
        transport; finally, connection_lost() will be called exactly once
        with either an exception object or None as an argument.

        State machine of calls:

          start -> CM [-> DR*] [-> ER?] -> CL -> end

        * CM: connection_made()
        * DR: data_received()
        * ER: eof_received()
        * CL: connection_lost()
        """

    def data_received(self, data: bytes):
        """Called when some data is received.

        The argument is a bytes object.
        """

    def eof_received(self):
        """Called when the other end calls write_eof() or equivalent.

        If this returns a false value (including None), the transport
        will close itself.  If it returns a true value, closing the
        transport is up to the protocol.
        """

    def connection_made(self, transport: asyncio.Transport):
        """Called when a connection is made.

        The argument is the transport representing the pipe connection.
        To receive data, wait for data_received() calls.
        When the connection is closed, connection_lost() is called.
        """

    def connection_lost(self, exc):
        """Called when the connection is lost or closed.

        The argument is an exception object or None (the latter
        meaning a regular EOF is received or the connection was
        aborted or closed).
        """

    def pause_writing(self):
        """Called when the transport's buffer goes over the high-water mark.

        Pause and resume calls are paired -- pause_writing() is called
        once when the buffer goes strictly over the high-water mark
        (even if subsequent writes increases the buffer size even
        more), and eventually resume_writing() is called once when the
        buffer size reaches the low-water mark.

        Note that if the buffer size equals the high-water mark,
        pause_writing() is not called -- it must go strictly over.
        Conversely, resume_writing() is called when the buffer size is
        equal or lower than the low-water mark.  These end conditions
        are important to ensure that things go as expected when either
        mark is zero.

        NOTE: This is the only Protocol callback that is not called
        through EventLoop.call_soon() -- if it were, it would have no
        effect when it's most needed (when the app keeps writing
        without yielding until pause_writing() is called).
        """

    def resume_writing(self):
        """Called when the transport's buffer drains below the low-water mark.

        See pause_writing() for details.
        """