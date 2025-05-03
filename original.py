#Import necessary libraries
from infinity import InfinityBase  # Assuming this provides base classes or utilities for Disney Infinity
import threading  # For creating and managing threads
from collections import defaultdict  # For creating dictionaries with default values for missing keys


# Class to handle communication with the Disney Infinity base
class InfinityComms(threading.Thread):
    def __init__(self):
        """Initialize the InfinityComms object."""
        threading.Thread.__init__(self)  # Initialize as a thread
        self.device = self.initBase()  # Establish connection to the base
        self.finish = False  # Flag to signal thread termination
        self.pending_requests = {}  # Store pending requests and their corresponding Deferred objects
        self.message_number = 0  # Track message numbers for requests
        self.observers = []  # List of observers to be notified of tag changes

    def initBase(self):
        """Initialize the connection to the Disney Infinity base."""
        hidapi.hid_init()  # Initialize HIDAPI library
        device = hidapi.hid_open(0x0e6f, 0x0129)  # Open connection to the base using vendor and product IDs
        hidapi.hid_set_nonblocking(device, False)  # Set to blocking mode (wait for data)
        return device  # Return the device object

    def run(self):
        """Main loop for the communication thread."""
        while not self.finish:  # Continue running until finish flag is set
            line = hidapi.hid_read_timeout(self.device, 32, 3000)  # Read data from the base with timeout

            if not len(line):  # If no data received, continue to next iteration
                continue

            fields = [c for c in line]  # Convert data to a list of bytes
            if fields[0] == 0xaa:  # Check for response message type
                length = fields[1]  # Extract message length
                message_id = fields[2]  # Extract message ID
                if message_id in self.pending_requests:  # Check if message is a response to a pending request
                    deferred = self.pending_requests[message_id]  # Get the Deferred object for the request
                    deferred.resolve(fields[3:length + 2])  # Resolve the Deferred with the received data
                    del self.pending_requests[message_id]  # Remove the request from pending requests
                else:
                    self.unknown_message(line)  # Handle unknown message
            elif fields[0] == 0xab:  # Check for tag update message type
                self.notifyObservers()  # Notify observers of tag changes
            else:
                self.unknown_message(line)  # Handle unknown message

    def addObserver(self, object):
        """Add an observer to be notified of tag changes."""
        self.observers.append(object)  # Add the observer to the list

    def notifyObservers(self):
        """Notify all observers of tag changes."""
        for obs in self.observers:  # Iterate through observers
            obs.tagsUpdated()  # Call the tagsUpdated method on each observer

    def unknown_message(self, fields):
        """Handle unknown messages received from the base."""
        print("UNKNOWN MESSAGE RECEIVED ", fields)  # Print a message indicating an unknown message

    def next_message_number(self):
        """Generate the next message number for a request."""
        self.message_number = (self.message_number + 1) % 256  # Increment message number and wrap around at 256
        return self.message_number  # Return the next message number

    def send_message(self, command, data=[]):
        """Send a message to the Disney Infinity base."""
        message_id, message = self.construct_message(command, data)  # Construct the message
        result = Deferred()  # Create a Deferred object to represent the future result
        self.pending_requests[message_id] = result  # Store the Deferred object for the request
        hidapi.hid_write(self.device, message)  # Send the message to the base
        return Promise(result)  # Return a Promise object representing the eventual result

    def construct_message(self, command, data):
        """Construct a message to be sent to the base."""
        message_id = self.next_message_number()  # Get the next message number
        command_body = [command, message_id] + data  # Combine command, message ID, and data
        command_length = len(command_body)  # Calculate command length
        command_bytes = [0x00, 0xff, command_length] + command_body  # Add header bytes
        message = [0x00] * 33  # Create a message buffer with 33 bytes
        checksum = 0  # Initialize checksum
        for (index, byte) in enumerate(command_bytes):  # Iterate through command bytes
            message[index] = byte  # Store byte in the message buffer
            checksum = checksum + byte  # Update checksum
        message[len(command_bytes)] = checksum & 0xff  # Append checksum to the message
        return (message_id, map(chr, message))  # Return message ID and the constructed message


# Class to represent a deferred result of an asynchronous operation
class Deferred(object):
    def __init__(self):
        """Initialize the Deferred object."""
        self.event = threading.Event()  # Create an event to signal completion
        self.rejected = False  # Flag to indicate if the operation was rejected
        self.result = None  # Store the result of the operation

    def resolve(self, value):
        """Resolve the Deferred with a value."""
        self.rejected = False  # Set rejected flag to False
        self.result = value  # Store the result value
        self.event.set()  # Signal completion

    def wait(self):
        """Wait for the Deferred to be resolved."""
        while not self.event.is_set():  # While the event is not set (operation not completed)
            self.event.wait(3)  # Wait for 3 seconds or until the event is set


# Class to represent a promise of an eventual result
class Promise(object):
    def __init__(self, deferred):
        """Initialize the Promise object."""
        self.deferred = deferred  # Store the associated Deferred object

    def then(self, success, failure=None):
        """Register callbacks for success and failure."""
        def task():
            """Task to be executed when the Deferred is resolved."""
            try:
                self.deferred.wait()  # Wait for the Deferred to be resolved
                result = self.deferred.result  # Get the result
                success(result)  # Call the success callback with the result
            except Exception as ex:  # Handle exceptions
                if failure:  # If a failure callback is provided
                    failure(ex)  # Call the failure callback with the exception
                else:
                    print(ex.message)  # Otherwise, print the exception message

        threading.Thread(target=task).start()  # Start a new thread to execute the task
        return self  # Return the Promise object for chaining

    def wait(self):
        """Wait for the Promise to be fulfilled."""
        self.deferred.wait()  # Wait for the associated Deferred to be resolved


# Class to represent the Disney Infinity base and provide high-level interactions
class InfinityBase(object):
    def __init__(self):
        """Initialize the InfinityBase object."""
        self.comms = InfinityComms()  # Create an InfinityComms object for communication
        self.comms.addObserver(self)  # Register self as an observer for tag changes
        self.onTagsChanged = None  # Callback function to be executed when tags change

    def connect(self):
        """Connect to the Disney Infinity base."""
        self.comms.daemon = True  # Set communication thread as a daemon thread
        self.comms.start()  # Start the communication thread
        self.activate()  # Activate the base

    def disconnect(self):
        """Disconnect from the Disney Infinity base."""
        self.comms.finish = True  # Set finish flag to signal thread termination

    def activate(self):
        """Activate the Disney Infinity base."""
        activate_message = [0x28, 0x63, 0x29, 0x20, 0x44,
                            0x69, 0x73, 0x6e, 0x65, 0x79,
                            0x20, 0x32, 0x30, 0x31, 0x33]  # Activation message
        self.comms.send_message(0x80, activate_message)  # Send activation message

    def tagsUpdated(self):
        """Handle tag updates."""
        if self.onTagsChanged:  # If a callback function is set
            self.onTagsChanged()  # Execute the callback function

    def getAllTags(self, then):
        """Get all tags on the base."""

        def queryAllTags(idx):
            """Query for all tags based on their indices."""
            if len(idx) == 0:  # If no indices, return an empty dictionary
                then(dict())
            numberToGet = [0] * len(idx)  # Initialize a counter for tags to retrieve
            tagByPlatform = defaultdict(list)  # Create a dictionary to store tags by platform
            for (platform, tagIdx) in idx:  # Iterate through tag indices
                def fileTag(platform):
                    """Create a closure to file tags by platform."""

                    def inner(tag):
                        """Store the retrieved tag and notify when all tags are retrieved."""
                        tagByPlatform[platform].append(tag)  # Add tag to the dictionary
                        numberToGet.pop()  # Decrement tag counter
                        if len(numberToGet) == 0:  # If all tags are retrieved
                            then(dict(tagByPlatform))  # Call the callback with the tag dictionary

                    return inner  # Return the inner function

                self.getTag(tagIdx, fileTag(platform))  # Get the tag for the current index

        self.getTagIdx(queryAllTags)  # Get tag indices and then query for all tags

    def getTagIdx(self, then):
        """Get the indices of tags on the base."""

        def parseIndex(bytes):
            """Parse the received bytes to extract tag indices."""
            values = [((byte & 0xF0) >> 4, byte & 0x0F) for byte in bytes if byte != 0x09]  # Extract indices
            then(values)  # Call the callback with the extracted indices

        self.comms.send_message(0xa1).then(parseIndex)  # Send message to get tag indices and then parse