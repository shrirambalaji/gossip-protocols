using System;
using System.Collections.Generic;
using System.Collections.Concurrent;
using System.Linq;
using System.Threading.Tasks;
using System.Text.Json;

namespace GossipProtocol
{
    /// <summary>
    /// Represents different types of requests in the gossip protocol
    /// </summary>
    public abstract class Request
    {
        public class Read : Request { }
        
        public class ReadOk : Request 
        { 
            public List<ulong> Messages { get; set; } = new();
        }
        
        public class Broadcast : Request 
        { 
            public ulong Message { get; set; }
        }
        
        public class Topology : Request 
        { 
            public Dictionary<string, List<string>> TopologyMap { get; set; } = new();
        }
    }

    /// <summary>
    /// Represents a message in the Maelstrom protocol
    /// </summary>
    public class Message
    {
        public string Source { get; set; } = "";
        public string Destination { get; set; } = "";
        public JsonElement Body { get; set; }
    }

    /// <summary>
    /// Mock runtime interface for demonstration purposes
    /// In a real implementation, this would handle network communication
    /// </summary>
    public interface IRuntime
    {
        string NodeId { get; }
        Task ReplyAsync(Message request, Request response);
        Task ReplyOkAsync(Message request);
        void ExecuteRpc(string targetNode, Request request);
        void Exit(Message request);
    }   
	
	/// <summary>
	/// Thread-safe state container for the gossip node
	/// </summary>
	public class NodeState
	{
		private readonly ConcurrentDictionary<long, bool> seen = new();
		private readonly object lockObject = new();
		private List<string> neighbours = new();

		public IEnumerable<long> SeenMessages => seen.Keys; public List<string> Neighbours
		{
			get
			{
				lock (lockObject)
				{
					return new List<string>(neighbours);
				}
			}
			set
			{
				lock (lockObject)
				{
					neighbours = new List<string>(value);
				}
			}
		}

		public bool TryAdd(long value)
		{
			return seen.TryAdd(value, true);
		}

		public List<long> GetSnapshot()
		{
			return seen.Keys.ToList();
		}
	}

    /// <summary>
    /// C# implementation of the Gossip Protocol Node
    /// 
    /// This demonstrates the same concepts as the Rust version:
    /// - Maintains a set of seen messages to avoid duplicates
    /// - Randomly selects a subset of neighbors for gossip propagation
    /// - Handles topology updates to manage neighbor relationships
    /// - Provides read access to the current state
    /// </summary>
    public class GossipNode
    {
        /// <summary>
        /// The number of random peers to select for gossiping
        /// </summary>
        private const int RandomPeerCount = 3;
        private readonly NodeState state;
        private readonly Random random;

        public GossipNode(List<string> neighbours)
        {
            state = new NodeState { Neighbours = neighbours };
            random = new Random();
        }

        /// <summary>
        /// Main message processing method - equivalent to the Rust process() method
        /// </summary>
        public async Task ProcessAsync(IRuntime runtime, Message request)
        {
            // In a real implementation, you'd deserialize the message body
            // based on the message type. This is simplified for demonstration.
            
            var messageType = GetMessageType(request.Body);
            
            switch (messageType)
            {
                case "read":
                    await HandleReadAsync(runtime, request);
                    break;
                    
                case "broadcast":
                    await HandleBroadcastAsync(runtime, request);
                    break;
                    
                case "topology":
                    await HandleTopologyAsync(runtime, request);
                    break;
                    
                default:
                    runtime.Exit(request);
                    break;
            }
        }        /// <summary>
        /// Handles read requests by returning all seen messages
        /// </summary>
        private async Task HandleReadAsync(IRuntime runtime, Message request)
        {
            var snapshot = state.GetSnapshot();
            var response = new Request.ReadOk { Messages = snapshot };
            await runtime.ReplyAsync(request, response);
        }

        /// <summary>
        /// Handles broadcast requests - the core of the gossip protocol
        /// 
        /// Each broadcast:
        /// 1. Checks if we've seen this message before
        /// 2. If new, adds it to our seen set
        /// 3. Randomly selects neighbors to forward the message to
        /// 4. Sends the message to those neighbors
        /// </summary>
        private async Task HandleBroadcastAsync(IRuntime runtime, Message request)
        {
            // Extract the message value from the request body
            var messageValue = ExtractMessageValue(request.Body);
              // Only propagate if this is a new message
            if (state.TryAdd(messageValue))
            {
                // Get current neighbors and shuffle them
                var neighbours = state.Neighbours;
                var shuffledNeighbours = neighbours.OrderBy(x => random.Next()).ToList();

                // Send to a random subset of neighbors (gossip propagation)
                foreach (var node in shuffledNeighbours.Take(RandomPeerCount))
                {
                    var broadcastRequest = new Request.Broadcast { Message = messageValue };
                    runtime.ExecuteRpc(node, broadcastRequest);
                }
            }

            // acknowledge the broadcast request
            await runtime.ReplyOkAsync(request);
        }

        /// <summary>
        /// Handles topology updates to manage neighbor relationships
        /// </summary>
        private async Task HandleTopologyAsync(IRuntime runtime, Message request)
        {
            var topology = ExtractTopology(request.Body);
              if (topology.TryGetValue(runtime.NodeId, out var neighbours))
            {
                state.Neighbours = neighbours;
                Console.WriteLine($"My neighbours are: {string.Join(", ", neighbours)}");
            }
            
            await runtime.ReplyOkAsync(request);
        }

        /// <summary>
        /// Helper methods for message parsing (simplified for demonstration)
        /// In a real implementation, these would use proper JSON deserialization
        /// </summary>
        private string GetMessageType(JsonElement body)
        {
            return "broadcast"; // placeholder
        }

        private ulong ExtractMessageValue(JsonElement body)
        {
            return 42; // placeholder
        }

        private Dictionary<string, List<string>> ExtractTopology(JsonElement body)
        {
            // Simplified - in reality you'd parse the JSON to extract the topology
            return new Dictionary<string, List<string>>(); // placeholder
        }
    }

    /// <summary>
    /// Example usage and comparison with Rust version
    /// </summary>
    public class Program
    {
        public static void Main()
        {}
    }
}
